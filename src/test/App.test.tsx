import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, it, expect } from 'vitest';
import App from '../App';
import type { Mapping, Preview, Transport, View } from '../types';

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
  const emptyDeck = { ...structuredClone(view.decks[0]), mapping: null };
  const emptyQueue = { ...structuredClone(view.queues.deck), cardIds: [], remainingNew: 0 };
  const transport: Transport = {
    async command<T>(request: Record<string, unknown>): Promise<T> {
      requests.push(request);
      if (request.type === 'previewImport') return structuredClone(responses.preview) as T;
      if (request.type === 'createDeck') {
        view.decks.push({ ...emptyDeck, id: 'created', name: String(request.name).trim() });
        view.queues.created = structuredClone(emptyQueue);
        view.revision += 1;
        return 'created' as T;
      }
      if (request.type === 'applyImport') {
        const requested = requests.filter((r) => r.type === 'previewImport').at(-1)!;
        view.decks.find((d) => d.id === requested.deckId)!.mapping = structuredClone(
          requested.mapping as Mapping,
        );
        view.revision += 1;
        return null as T;
      }
      if (request.type === 'deleteDeck') {
        view.decks = view.decks.filter((deck) => deck.id !== request.deckId);
        view.cards = view.cards.filter((card) => card.deckId !== request.deckId);
        delete view.queues[String(request.deckId)];
        view.revision += 1;
        return null as T;
      }
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
  it('preserves applied matching preferences through reopen without leaking them to another deck', async () => {
    const { transport, view, responses } = harness();
    view.decks.push({
      ...view.decks[0],
      id: 'other',
      name: 'IDで照合',
      mapping: {
        question: 1,
        answer: 2,
        explanation: null,
        id: 0,
        choices: [3],
        choiceSeparator: '|',
      },
    });
    transport.pickCsv = async () => ({
      token: 'source',
      headers: ['ID', '問題', '答え', '選択肢'],
      rows: [['1', 'Q', 'A', 'A|B']],
    });
    responses.preview = {
      token: 'preview',
      deckId: 'deck',
      changes: [],
      errors: [],
      missing: 0,
      mergedRows: 0,
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    await user.selectOptions(screen.getByRole('combobox', { name: /照合用ID/ }), '');
    await user.click(screen.getByRole('button', { name: '選択肢を使わない' }));
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    await user.click(screen.getByRole('button', { name: '内容を適用する' }));
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByRole('combobox', { name: /照合用ID/ })).toHaveValue('');
    expect(screen.getByRole('checkbox', { name: '4. 選択肢' })).not.toBeChecked();
    await user.click(screen.getByRole('button', { name: '閉じる' }));
    await user.click(screen.getByRole('button', { name: 'IDで照合 0' }));
    await user.click(screen.getByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByRole('combobox', { name: /照合用ID/ })).toHaveValue('0');
    expect(screen.getByRole('checkbox', { name: '4. 選択肢' })).toBeChecked();
    expect(screen.getByLabelText('選択肢の区切り文字')).toHaveValue('|');
  });

  it('imports into the newly created deck instead of the previously selected deck', async () => {
    const { transport, requests, responses, view } = harness();
    view.decks[0].mapping = {
      question: 1,
      answer: 2,
      explanation: null,
      id: null,
      choices: [],
      choiceSeparator: null,
    };
    transport.pickCsv = async () => ({
      token: 'new',
      headers: ['ID', '問題', '答え'],
      rows: [['1', 'Q', 'A']],
    });
    responses.preview = {
      token: 'preview',
      deckId: 'created',
      changes: [],
      errors: [],
      missing: 0,
      mergedRows: 0,
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '単語帳を作成' }));
    await user.type(screen.getByLabelText('単語帳の名前'), '新しく作成');
    await user.click(screen.getByRole('button', { name: '作成してCSVを選ぶ' }));
    expect(
      await screen.findByRole('dialog', { name: 'CSVを取り込む · 新しく作成' }),
    ).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByRole('combobox', { name: /照合用ID/ })).toHaveValue('0');
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    expect(requests.filter((r) => r.type === 'previewImport').at(-1)?.deckId).toBe('created');
  });

  it('warns when the previous ID matching mode cannot be inferred from the new layout', async () => {
    const { transport, view } = harness();
    view.decks[0].mapping = {
      question: 0,
      answer: 1,
      explanation: null,
      id: 2,
      choices: [],
      choiceSeparator: null,
    };
    transport.pickCsv = async () => ({
      token: 'reordered',
      headers: ['識別番号', '問題', '答え'],
      rows: [['001', 'Q', 'A']],
    });
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByRole('combobox', { name: /照合用ID/ })).toHaveValue('');
    expect(screen.getByText(/前回の照合方法から変わっています/)).toBeInTheDocument();
    await user.selectOptions(screen.getByRole('combobox', { name: /照合用ID/ }), '0');
    expect(screen.queryByText(/前回の照合方法から変わっています/)).not.toBeInTheDocument();
  });

  it('keeps question matching when reimporting a deck that explicitly does not use IDs', async () => {
    const { transport, view } = harness();
    view.decks[0].mapping = {
      question: 1,
      answer: 2,
      explanation: null,
      id: null,
      choices: [],
      choiceSeparator: null,
    };
    transport.pickCsv = async () => ({
      token: 'again',
      headers: ['ID', '問題', '答え'],
      rows: [
        ['1', 'Q1', 'A'],
        ['1', 'Q2', 'B'],
      ],
    });
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByLabelText(/照合用ID/)).toHaveValue('');
  });

  it.each([
    { headers: ['ID', 'Term', 'Definition'] },
    { headers: ['ID', '問題', '日本語'] },
    { headers: ['answer', 'question', 'extra'] },
    { headers: ['解説', '英単語', '日本語', 'ID'] },
    { headers: ['ID', 'answer'] },
  ])('never assigns the same CSV column to multiple roles: $headers', async ({ headers }) => {
    const { transport } = harness();
    transport.pickCsv = async () => ({
      token: 'columns',
      headers,
      rows: [headers.map((_, i) => `value${i}`)],
    });
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    const columns = [/^問題$/, /^答え$/, /解説/, /照合用ID/]
      .map((label) => (screen.getByRole('combobox', { name: label }) as HTMLSelectElement).value)
      .filter((value) => value !== '');
    expect(new Set(columns).size).toBe(columns.length);
  });

  it('keeps the import target when the available decks change during column selection', async () => {
    const { transport, requests, view, responses } = harness();
    transport.pickCsv = async () => ({
      token: 'target',
      headers: ['問題', '答え'],
      rows: [['Q', 'A']],
    });
    responses.preview = {
      token: 'preview',
      deckId: 'deck',
      changes: [],
      errors: [],
      missing: 0,
      mergedRows: 0,
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    view.decks = [{ ...view.decks[0], id: 'other', name: '別の単語帳' }];
    view.cards = [];
    view.queues = {};
    view.revision += 1;
    fireEvent.focus(window);
    await screen.findAllByText(/別の単語帳/);
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    expect(requests.filter((r) => r.type === 'previewImport').at(-1)?.deckId).toBe('deck');
    expect(screen.getByRole('dialog')).toHaveAccessibleName('CSVを取り込む · 基礎の単語');
  });

  it('lets the user omit an automatically selected choice column', async () => {
    const { transport, requests, responses } = harness();
    transport.pickCsv = async () => ({
      token: 'optional',
      headers: ['問題', '答え', '選択肢'],
      rows: [['Q', 'A', 'unused']],
    });
    responses.preview = {
      token: 'preview',
      deckId: 'deck',
      changes: [],
      errors: [],
      missing: 0,
      mergedRows: 0,
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    await user.selectOptions(screen.getByLabelText('選択肢セルの読み方'), 'custom');
    await user.click(screen.getByRole('button', { name: '選択肢を使わない' }));
    expect(screen.getByText('選択肢なしで取り込みます。')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: '3. 選択肢' })).not.toBeChecked();
    expect(screen.queryByLabelText('選択肢の区切り文字')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    expect(requests).toContainEqual({
      type: 'previewImport',
      deckId: 'deck',
      sourceToken: 'optional',
      mapping: {
        question: 0,
        answer: 1,
        explanation: null,
        id: null,
        choices: [],
        choiceSeparator: null,
      },
    });
  });

  it('explicitly switches an ID conflict to question matching without applying or losing choices', async () => {
    const { transport, requests, responses } = harness();
    transport.pickCsv = async () => ({
      token: 'csv',
      headers: ['ID', '問題', '答え', '選択肢'],
      rows: [
        ['1', 'Q1', 'A', 'A|B'],
        ['1', 'Q2', 'B', 'A|B'],
      ],
    });
    responses.preview = {
      token: 'conflict',
      deckId: 'deck',
      changes: [],
      missing: 0,
      mergedRows: 0,
      errors: [{ line: 3, message: '2行目と同じIDですが、問題文が異なります。' }],
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    await user.selectOptions(screen.getByLabelText('選択肢セルの読み方'), 'custom');
    await user.type(screen.getByLabelText('選択肢の区切り文字'), '|');
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('問題文が異なります');
    expect(screen.getByRole('button', { name: '内容を適用する' })).toBeDisabled();
    expect(requests.filter((r) => r.type === 'previewImport')).toHaveLength(1);
    responses.preview = { ...responses.preview, token: 'by-question', errors: [] };
    await user.click(screen.getByRole('button', { name: 'IDを使わず問題文でプレビュー' }));
    await waitFor(() =>
      expect(requests).toContainEqual({
        type: 'previewImport',
        deckId: 'deck',
        sourceToken: 'csv',
        mapping: {
          question: 1,
          answer: 2,
          explanation: null,
          id: null,
          choices: [3],
          choiceSeparator: '|',
        },
      }),
    );
    expect(screen.getByText('照合方法：問題文の完全一致')).toBeInTheDocument();
    expect(requests.some((r) => r.type === 'applyImport')).toBe(false);
  });

  it('imports a two-column CSV without requiring choices', async () => {
    const { transport, requests, responses } = harness();
    transport.pickCsv = async () => ({
      token: 'plain',
      headers: ['問題', '答え'],
      rows: [['Q', 'A']],
    });
    responses.preview = {
      token: 'preview',
      deckId: 'deck',
      changes: [],
      errors: [],
      missing: 0,
      mergedRows: 0,
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: 'CSVを取り込む' }));
    await user.click(screen.getByRole('button', { name: 'ファイルを選ぶ' }));
    expect(screen.getByText('選択肢なしで取り込みます。')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '更新内容をプレビュー' }));
    await waitFor(() =>
      expect(requests).toContainEqual({
        type: 'previewImport',
        deckId: 'deck',
        sourceToken: 'plain',
        mapping: {
          question: 0,
          answer: 1,
          explanation: null,
          id: null,
          choices: [],
          choiceSeparator: null,
        },
      }),
    );
  });

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

describe('deck deletion', () => {
  it('keeps the confirmed target when a refresh changes the available decks', async () => {
    const { transport, requests, view } = harness();
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '学習設定' }));
    await user.click(screen.getByRole('button', { name: '単語帳を削除' }));
    view.decks = [{ ...view.decks[0], id: 'other', name: '残す単語帳' }];
    view.cards = [];
    view.queues = {};
    view.revision += 1;
    fireEvent.focus(window);
    await screen.findAllByText('残す単語帳');
    expect(screen.getByRole('dialog', { name: '単語帳を削除しますか？' })).toHaveTextContent(
      '基礎の単語',
    );
    await user.click(screen.getByRole('button', { name: 'この単語帳を削除する' }));
    await waitFor(() =>
      expect(requests.filter((r) => r.type === 'deleteDeck')).toEqual([
        { type: 'deleteDeck', deckId: 'deck' },
      ]),
    );
    expect(view.decks[0].id).toBe('other');
  });

  it('keeps the deck and confirmation visible when saving the deletion fails', async () => {
    const { transport, view } = harness();
    const command = transport.command;
    transport.command = async <T,>(request: Record<string, unknown>): Promise<T> => {
      if (request.type === 'deleteDeck') throw new Error('保存処理に失敗しました。');
      return command<T>(request);
    };
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '学習設定' }));
    await user.click(screen.getByRole('button', { name: '単語帳を削除' }));
    await user.click(screen.getByRole('button', { name: 'この単語帳を削除する' }));
    for (const alert of await screen.findAllByRole('alert')) {
      expect(alert).toHaveTextContent('保存処理に失敗しました。');
    }
    expect(screen.getByRole('dialog', { name: '単語帳を削除しますか？' })).toBeInTheDocument();
    expect(screen.queryByText('単語帳を削除しました。')).not.toBeInTheDocument();
    expect(view.decks).toHaveLength(1);
  });

  it('requires confirmation, supports cancellation, and returns to the empty library after deleting the last deck', async () => {
    const { transport, requests } = harness();
    const user = userEvent.setup();
    render(<App transport={transport} />);
    await user.click(await screen.findByRole('button', { name: '学習設定' }));
    await user.click(screen.getByRole('button', { name: '単語帳を削除' }));
    expect(screen.getByRole('dialog', { name: '単語帳を削除しますか？' })).toHaveTextContent(
      '基礎の単語',
    );
    expect(requests.some((r) => r.type === 'deleteDeck')).toBe(false);
    await user.click(screen.getByRole('button', { name: 'キャンセル' }));
    expect(requests.some((r) => r.type === 'deleteDeck')).toBe(false);
    await user.click(screen.getByRole('button', { name: '学習設定' }));
    await user.click(screen.getByRole('button', { name: '単語帳を削除' }));
    await user.click(screen.getByRole('button', { name: 'この単語帳を削除する' }));
    await waitFor(() =>
      expect(requests.filter((r) => r.type === 'deleteDeck')).toEqual([
        { type: 'deleteDeck', deckId: 'deck' },
      ]),
    );
    expect(await screen.findByRole('button', { name: '最初の単語帳を作る' })).toBeInTheDocument();
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
    expect(screen.queryByRole('list', { name: '選択肢' })).not.toBeInTheDocument();
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
