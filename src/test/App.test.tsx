import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect } from 'vitest';
import App from '../App';
import type { Transport, View } from '../types';

function harness() {
  const requests: Record<string, unknown>[] = [];
  const view: View = {
    revision: 1,
    settings: { timezone: 'Asia/Tokyo', dayStartHour: 4 },
    reviewCount: 0,
    canUndo: false,
    decks: [
      {
        id: 'deck',
        name: '基礎の単語',
        createdAt: 0,
        retention: 0.9,
        newLimit: 20,
        reviewLimit: null,
        parameters: [],
        parameterVersion: 'FSRS-6',
        mapping: null,
      },
    ],
    cards: [
      {
        id: 'card',
        deckId: 'deck',
        sourceId: null,
        question: '<img src=x onerror=alert(1)>',
        answer: '答えの本文',
        explanation: '解説の本文',
        createdAt: 0,
        schedule: {
          stability: null,
          difficulty: null,
          lastReview: null,
          due: null,
          reps: 0,
          lapses: 0,
        },
      },
    ],
    queues: {
      deck: {
        day: '2026-09-09',
        cardIds: ['card'],
        remainingNew: 1,
        dueReviews: 0,
        newUsed: 0,
        reviewUsed: 0,
        newLimit: 20,
        reviewLimit: null,
        newBonus: 0,
        reviewBonus: 0,
      },
    },
  };
  const transport: Transport = {
    async command<T>(request: Record<string, unknown>): Promise<T> {
      requests.push(request);
      if (request.type === 'grade') {
        view.queues.deck.cardIds = [];
        view.canUndo = true;
        if (request.rating === 1) {
          const now = Math.floor(Date.now() / 1000);
          view.cards[0].schedule = {
            stability: 1,
            difficulty: 5,
            lastReview: now,
            due: now + 60,
            reps: 1,
            lapses: 1,
          };
        }
        return null as T;
      }
      if (request.type === 'view') return structuredClone(view) as T;
      return null as T;
    },
    pickCsv: async () => null,
    exportBackup: async () => false,
    previewRestore: async () => null,
  };
  return { transport, requests };
}

describe('learning flow', () => {
  it('hides answers until revealed, renders content as text, then sends one rating', async () => {
    const { transport, requests } = harness();
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '学習をはじめる' }));
    expect(screen.queryByText('答えの本文')).not.toBeInTheDocument();
    expect(screen.getByText('<img src=x onerror=alert(1)>')).toBeInTheDocument();
    expect(document.querySelector('img')).toBeNull();
    await user.keyboard(' ');
    expect(screen.getByText('答えの本文')).toBeInTheDocument();
    expect(screen.getByText('解説の本文')).toBeInTheDocument();
    await user.keyboard('3');
    await waitFor(() =>
      expect(requests.filter((r) => r.type === 'grade')).toEqual([
        { type: 'grade', cardId: 'card', rating: 3 },
      ]),
    );
    expect(await screen.findByText('今日の学習が終わりました')).toBeInTheDocument();
  });

  it('keeps today-only increases separate from permanent settings', async () => {
    const { transport, requests } = harness();
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '今日だけ上乗せ' }));
    const input = screen.getByLabelText('今日の新規に追加する枚数');
    await user.clear(input);
    await user.type(input, '10');
    await user.click(screen.getByRole('button', { name: '上乗せする' }));
    await waitFor(() =>
      expect(requests).toContainEqual({ type: 'addBonus', deckId: 'deck', new: 10, review: 0 }),
    );
    expect(requests.some((r) => r.type === 'configureDeck')).toBe(false);
  });

  it('shows a short wait instead of completion after a forgotten card', async () => {
    const { transport } = harness();
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '学習をはじめる' }));
    await user.keyboard(' ');
    await user.keyboard('1');
    expect(await screen.findByText('少し待って、もう一度')).toBeInTheDocument();
    expect(screen.queryByText('今日の学習が終わりました')).not.toBeInTheDocument();
  });
});
