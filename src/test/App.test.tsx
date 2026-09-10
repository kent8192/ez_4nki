import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect } from 'vitest';
import App from '../App';
import type { Preview, Transport, View } from '../types';

function harness() {
  const requests: Record<string, unknown>[] = [];
  const responses: { preview: Preview | null } = { preview: null };
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
        choices: [],
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
      if (request.type === 'previewImport') return structuredClone(responses.preview) as T;
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
  return { transport, requests, view, responses };
}

describe('CSV choices and consolidation', () => {
  it('maps one choice column, passes the delimiter, and previews merged rows before applying', async () => {
    const { transport, requests, view, responses } = harness();
    transport.pickCsv = async () => ({
      token: 'csv',
      headers: ['問題', '答え', '選択肢', '補助情報'],
      rows: [
        ['Q', 'A', 'A. one|B. two', 'ignored'],
        ['Q', 'A', 'A. one|B. two', 'ignored'],
      ],
    });
    responses.preview = {
      token: 'preview',
      deckId: 'deck',
      mergedRows: 1,
      missing: 0,
      errors: [],
      changes: [
        {
          line: 2,
          sourceLines: [2, 3],
          kind: 'new',
          before: null,
          notes: ['異なる解説を2件まとめています。内容を確認してください。'],
          after: {
            ...view.cards[0],
            question: 'Q',
            answer: 'A',
            choices: [
              { label: '', text: 'A. one' },
              { label: '', text: 'B. two' },
            ],
          },
        },
      ],
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByRole('checkbox', { name: '3. 選択肢' })).toBeChecked();
    expect(screen.getByRole('checkbox', { name: '1. 問題' })).toBeDisabled();
    expect(screen.getByLabelText('選択肢セルの読み方')).toHaveValue('whole');
    await user.selectOptions(screen.getByLabelText('選択肢セルの読み方'), 'custom');
    await user.type(screen.getByLabelText('選択肢の区切り文字'), '|');
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    expect(await screen.findByText('重複 1行を統合')).toBeInTheDocument();
    expect(screen.getByText('2・3行目')).toBeInTheDocument();
    expect(screen.getByText('2行 → 1枚')).toBeInTheDocument();
    expect(screen.getByText(/異なる解説を2件まとめています/)).toBeInTheDocument();
    expect(screen.getByRole('list', { name: '選択肢' })).toHaveTextContent('B. two');
    expect(requests).toContainEqual({
      type: 'previewImport',
      deckId: 'deck',
      sourceToken: 'csv',
      mapping: {
        question: 0,
        answer: 1,
        explanation: null,
        id: null,
        choices: [2],
        choiceSeparator: '|',
      },
    });
    expect(requests.some((r) => r.type === 'applyImport')).toBe(false);
    await user.click(screen.getByRole('button', { name: '内容を適用する' }));
    await waitFor(() => expect(requests).toContainEqual({ type: 'applyImport', token: 'preview' }));
  });

  it('lets the user edit a choice without dropping the remaining choices', async () => {
    const { transport, requests, view } = harness();
    view.cards[0].choices = [
      { label: 'A', text: 'first' },
      { label: 'B', text: 'second' },
    ];
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'カード一覧' }));
    await user.click(screen.getByRole('button', { name: '編集' }));
    const choice = screen.getByLabelText('選択肢 1 の内容');
    await user.clear(choice);
    await user.type(choice, 'changed');
    await user.click(screen.getByRole('button', { name: '変更を保存' }));
    await waitFor(() =>
      expect(requests).toContainEqual({
        type: 'editCard',
        cardId: 'card',
        question: view.cards[0].question,
        answer: view.cards[0].answer,
        explanation: view.cards[0].explanation,
        choices: [
          { label: 'A', text: 'changed' },
          { label: 'B', text: 'second' },
        ],
      }),
    );
  });
});

describe('learning flow', () => {
  it('shows a single-column choice block before revealing the answer as plain text', async () => {
    const { transport, requests, view } = harness();
    view.cards[0].choices = [{ label: '選択肢', text: 'A. first\nB. <img src=x>' }];
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '学習をはじめる' }));
    const choices = screen.getByRole('list', { name: '選択肢' });
    expect(choices).toHaveTextContent('A. first B. <img src=x>');
    expect(choices.querySelector('.plain-text:last-child')?.textContent).toBe(
      'A. first\nB. <img src=x>',
    );
    expect(document.querySelector('img')).toBeNull();
    expect(screen.queryByText('答えの本文')).not.toBeInTheDocument();
    expect(requests.some((r) => r.type === 'grade')).toBe(false);
    await user.keyboard(' ');
    expect(screen.getByText('答えの本文')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: '選択肢' })).toBeInTheDocument();
  });
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
