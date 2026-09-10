import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import {
  BookOpen,
  Plus,
  Upload,
  ArrowUpRight,
  ArrowLeft,
  ArrowRight,
  Check,
  ChevronRight,
  X,
  Settings2,
  ShieldCheck,
  BarChart3,
  LibraryBig,
  LayoutDashboard,
  RotateCcw,
  Download,
  Search,
  Sparkles,
  Clock3,
  FileSpreadsheet,
  LockKeyhole,
} from 'lucide-react';
import type {
  Analytics,
  Card,
  Choice,
  CurvePoint,
  Mapping,
  Optimization,
  Preview,
  RestorePreview,
  Source,
  Transport,
  View,
} from './types';
import './style.css';

const percent = (n: number) => `${Math.round(n * 1000) / 10}%`;
const date = (n: number | null) =>
  n == null
    ? '未学習'
    : new Date(n * 1000).toLocaleDateString('ja-JP', { month: 'short', day: 'numeric' });
type ModalName =
  'create' | 'import' | 'study' | 'bonus' | 'deckSettings' | 'backup' | 'restore' | 'edit' | null;

function Choices({ choices }: { choices: Choice[] }) {
  if (!choices.length) return null;
  return (
    <ul className="choice-list" aria-label="選択肢">
      {choices.map((choice, i) => (
        <li key={i}>
          <span className="choice-label plain-text">{choice.label || `選択肢 ${i + 1}`}</span>
          <span className="plain-text">{choice.text}</span>
        </li>
      ))}
    </ul>
  );
}

function Modal({
  title,
  children,
  close,
  wide = false,
}: {
  title: string;
  children: ReactNode;
  close: () => void;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => previous?.focus();
  }, []);
  return (
    <div className="overlay">
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={`modal ${wide ? 'wide' : ''}`}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.stopPropagation();
            close();
          }
          if (e.key === 'Tab') {
            const controls = ref.current?.querySelectorAll<HTMLElement>(
              'button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]',
            );
            if (!controls?.length) return;
            const first = controls[0],
              last = controls[controls.length - 1];
            if (
              e.shiftKey &&
              (document.activeElement === first || document.activeElement === ref.current)
            ) {
              e.preventDefault();
              last.focus();
            } else if (!e.shiftKey && document.activeElement === last) {
              e.preventDefault();
              first.focus();
            }
          }
        }}
      >
        <header className="modal-header">
          <h2>{title}</h2>
          <button className="icon-button" aria-label="閉じる" onClick={close}>
            <X size={20} />
          </button>
        </header>
        {children}
      </div>
    </div>
  );
}

function Curve({ points }: { points: CurvePoint[] }) {
  if (!points.length)
    return <div className="chart-empty">学習を始めると、推定忘却曲線を表示します。</div>;
  const path = points
    .map((p, i) => `${i ? 'L' : 'M'}${40 + p.days * 16},${175 - p.retention * 140}`)
    .join(' ');
  return (
    <svg viewBox="0 0 560 215" className="chart" role="img" aria-label="今から30日間の推定忘却曲線">
      {[0, 0.5, 1].map((n) => (
        <g key={n}>
          <line x1="40" x2="520" y1={175 - n * 140} y2={175 - n * 140} className="grid-line" />
          <text x="0" y={180 - n * 140}>
            {n * 100}%
          </text>
        </g>
      ))}
      <path d={`${path} L520,175 L40,175 Z`} fill="#e4f0eb" />
      <path d={path} fill="none" stroke="#23715c" strokeWidth="3" />
      {points.map((p) => (
        <circle
          key={p.days}
          cx={40 + p.days * 16}
          cy={175 - p.retention * 140}
          r="4"
          fill="#23715c"
          className="chart-dot"
        >
          <title>
            {p.days}日後：{percent(p.retention)}
          </title>
        </circle>
      ))}
      <text x="40" y="202">
        今日
      </text>
      <text x="268" y="202">
        15日後
      </text>
      <text x="482" y="202">
        30日後
      </text>
    </svg>
  );
}

export default function App({ transport }: { transport: Transport }) {
  const [view, setView] = useState<View | null>(null);
  const [selected, setSelected] = useState('');
  const [page, setPage] = useState('home');
  const [modal, setModal] = useState<ModalName>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [name, setName] = useState('');
  const [query, setQuery] = useState('');
  const [source, setSource] = useState<Source | null>(null);
  const [encoding, setEncoding] = useState('utf-8');
  const [mapping, setMapping] = useState<Mapping>({
    question: 0,
    answer: 1,
    explanation: null,
    id: null,
    choices: [],
    choiceSeparator: null,
  });
  const [preview, setPreview] = useState<Preview | null>(null);
  const [revealed, setRevealed] = useState(false);
  const [newBonus, setNewBonus] = useState(10);
  const [reviewBonus, setReviewBonus] = useState(0);
  const [newLimit, setNewLimit] = useState(20);
  const [reviewLimit, setReviewLimit] = useState('');
  const [retention, setRetention] = useState(90);
  const [dayStart, setDayStart] = useState(4);
  const [timezone, setTimezone] = useState('Asia/Tokyo');
  const [passphrase, setPassphrase] = useState('');
  const [passConfirm, setPassConfirm] = useState('');
  const [restore, setRestore] = useState<RestorePreview | null>(null);
  const [edit, setEdit] = useState<Card | null>(null);
  const [analytics, setAnalytics] = useState<Analytics | null>(null);
  const [curveCard, setCurveCard] = useState('');
  const [curve, setCurve] = useState<CurvePoint[]>([]);
  const [optimization, setOptimization] = useState<Optimization | null>(null);
  const busyRef = useRef(false);
  const deck = view?.decks.find((d) => d.id === selected) ?? view?.decks[0];
  const cards = view?.cards.filter((c) => c.deckId === deck?.id) ?? [];
  const queue = deck ? view?.queues[deck.id] : undefined;
  const currentCard = cards.find((c) => c.id === queue?.cardIds[0]);
  const nextRepetition = cards
    .map((c) => c.schedule)
    .filter((s) => s.due && s.lastReview && s.due > Date.now() / 1000 && s.due - s.lastReview <= 60)
    .reduce<number | undefined>((next, s) => Math.min(next ?? Infinity, s.due!), undefined);
  const reportError = (e: unknown) =>
    setError(
      typeof e === 'string'
        ? e
        : e instanceof Error
          ? e.message
          : '操作に失敗しました。もう一度お試しください。',
    );
  const refresh = useCallback(async () => {
    const v = await transport.command<View>({ type: 'view' });
    setView((previous) => (previous && previous.revision > v.revision ? previous : v));
  }, [transport]);
  useEffect(() => {
    void refresh().catch(reportError);
  }, [refresh]);
  useEffect(() => {
    if (!view) return;
    let stopped = false;
    let timer: number;
    const schedule = () => {
      // Align refresh with the next minute so the configured day boundary is
      // reflected immediately, and wake for a pending one-minute repetition.
      const dueDelay = nextRepetition ? nextRepetition * 1000 - Date.now() : Infinity;
      const delay = Math.max(
        50,
        Math.min(30000, 60000 - (Date.now() % 60000), dueDelay > 0 ? dueDelay : Infinity),
      );
      timer = window.setTimeout(() => {
        void refresh()
          .catch(reportError)
          .finally(() => {
            if (!stopped) schedule();
          });
      }, delay);
    };
    const onFocus = () => void refresh().catch(reportError);
    schedule();
    window.addEventListener('focus', onFocus);
    return () => {
      stopped = true;
      clearTimeout(timer);
      window.removeEventListener('focus', onFocus);
    };
  }, [view !== null, nextRepetition, refresh]);
  useEffect(() => {
    if (view) {
      setDayStart(view.settings.dayStartHour);
      setTimezone(view.settings.timezone);
    }
  }, [view?.settings.dayStartHour, view?.settings.timezone]);
  useEffect(() => {
    setRevealed(false);
  }, [currentCard?.id]);

  const run = useCallback(async (action: () => Promise<void>) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    setError('');
    setNotice('');
    try {
      await action();
    } catch (e) {
      reportError(e);
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, []);
  const close = () => {
    if (busyRef.current) return;
    if (modal === 'import') void transport.command({ type: 'discardImport' }).catch(reportError);
    if (modal === 'restore') void transport.command({ type: 'discardRestore' }).catch(reportError);
    setModal(null);
    setSource(null);
    setPreview(null);
    setRestore(null);
    setPassphrase('');
    setPassConfirm('');
  };
  const mutate = (request: Record<string, unknown>, message?: string, stay = false) =>
    run(async () => {
      await transport.command(request);
      await refresh();
      if (!stay) setModal(null);
      if (message) setNotice(message);
    });
  const grade = useCallback(
    (rating: number) => {
      if (!currentCard || !revealed || busyRef.current) return;
      void run(async () => {
        await transport.command({ type: 'grade', cardId: currentCard.id, rating });
        setRevealed(false);
        await refresh();
      });
    },
    [currentCard?.id, revealed, transport, run, refresh],
  );
  useEffect(() => {
    if (modal !== 'study') return;
    const onKey = (event: KeyboardEvent) => {
      if (
        event.repeat ||
        (event.target as HTMLElement)?.matches('input,textarea,select,[contenteditable="true"]')
      )
        return;
      if (event.code === 'Space' || event.key === ' ') {
        event.preventDefault();
        if (!busyRef.current) setRevealed(true);
      }
      if (revealed && /^[1-4]$/.test(event.key)) {
        event.preventDefault();
        grade(Number(event.key));
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [modal, revealed, grade]);

  const loadAnalytics = useCallback(async () => {
    if (!deck) return;
    setAnalytics(await transport.command<Analytics>({ type: 'analytics', deckId: deck.id }));
    setOptimization(await transport.command<Optimization>({ type: 'optimization' }));
  }, [deck?.id, transport]);
  useEffect(() => {
    setAnalytics(null);
    setCurve([]);
    setCurveCard('');
    if (page === 'analytics') void loadAnalytics().catch(reportError);
  }, [page, deck?.id, loadAnalytics]);
  useEffect(() => {
    if (!curveCard) {
      setCurve([]);
      return;
    }
    let cancelled = false;
    void transport
      .command<CurvePoint[]>({ type: 'curve', cardId: curveCard })
      .then((p) => {
        if (!cancelled) setCurve(p);
      })
      .catch(reportError);
    return () => {
      cancelled = true;
    };
  }, [curveCard, view?.revision, transport]);
  useEffect(() => {
    if (optimization?.status !== 'running') return;
    const timer = setInterval(
      () =>
        void transport
          .command<Optimization>({ type: 'optimization' })
          .then(setOptimization)
          .catch(reportError),
      700,
    );
    return () => clearInterval(timer);
  }, [optimization?.status, transport]);

  const openImport = () => {
    setModal('import');
    setSource(null);
    setPreview(null);
  };
  const chooseCsv = () =>
    run(async () => {
      const selectedSource = await transport.pickCsv(encoding);
      if (selectedSource) {
        setSource(selectedSource);
        setPreview(null);
        const find = (names: string[]) =>
          selectedSource.headers.findIndex((h) => names.includes(h.toLowerCase()));
        const q = find(['問題', 'question', 'front']),
          a = find(['答え', 'answer', 'back']),
          e = find(['解説', 'explanation']),
          id = find(['id']);
        setMapping({
          question: q >= 0 ? q : 0,
          answer: a >= 0 ? a : 1,
          explanation: e >= 0 ? e : null,
          id: id >= 0 ? id : null,
          choices: selectedSource.headers.flatMap((header, column) =>
            ['選択肢', 'choices', 'options'].includes(header.toLowerCase()) &&
            ![q >= 0 ? q : 0, a >= 0 ? a : 1, e, id].includes(column)
              ? [column]
              : [],
          ),
          choiceSeparator: null,
        });
      }
    });
  const openDeckSettings = () => {
    if (!deck) return;
    setRetention(Math.round(deck.retention * 1000) / 10);
    setNewLimit(deck.newLimit);
    setReviewLimit(deck.reviewLimit == null ? '' : String(deck.reviewLimit));
    setModal('deckSettings');
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">
            <BookOpen size={23} />
          </span>
          <div>
            単語帳<span>小さな復習を、毎日に。</span>
          </div>
        </div>
        <div className="workspace-label">MY LEARNING</div>
        <nav aria-label="メインナビゲーション">
          {[
            ['home', 'ホーム', LayoutDashboard],
            ['cards', 'カード一覧', LibraryBig],
            ['analytics', '学習の分析', BarChart3],
          ].map(([id, label, Icon]) => (
            <button
              key={String(id)}
              className={`nav-item ${page === id ? 'active' : ''}`}
              onClick={() => setPage(String(id))}
            >
              {typeof Icon !== 'string' && <Icon size={19} />}
              <span>{String(label)}</span>
              {page === id && <ChevronRight size={15} />}
            </button>
          ))}
        </nav>
        <div className="deck-heading">
          <span>単語帳</span>
          <button
            className="icon-button"
            aria-label="単語帳を作成"
            onClick={() => {
              setName('');
              setModal('create');
            }}
          >
            <Plus size={17} />
          </button>
        </div>
        <div className="deck-nav">
          {view?.decks.map((d) => (
            <button
              className={`deck-item ${deck?.id === d.id ? 'selected' : ''}`}
              key={d.id}
              onClick={() => {
                setSelected(d.id);
                setQuery('');
              }}
            >
              <span className="deck-dot" />
              <span>{d.name}</span>
              <small>{view.cards.filter((c) => c.deckId === d.id).length}</small>
            </button>
          ))}
          {!view?.decks.length && (
            <p className="side-empty">
              最初の単語帳を
              <br />
              作成しましょう。
            </p>
          )}
        </div>
        <div className="sidebar-footer">
          <div className="local-status">
            <ShieldCheck size={16} />
            <span>この端末だけに保存</span>
            <span className="status-dot" />
          </div>
          <button
            className={`nav-item ${page === 'settings' ? 'active' : ''}`}
            onClick={() => setPage('settings')}
          >
            <Settings2 size={18} />
            設定とバックアップ
          </button>
          <small>通信なし · 手動更新</small>
        </div>
      </aside>
      <main className="main-area">
        <header className="topbar">
          <span>
            私の学習 <ChevronRight size={14} />{' '}
            <strong>
              {page === 'settings' ? '設定とバックアップ' : (deck?.name ?? 'はじめての単語帳')}
            </strong>
          </span>
          <span className="date-label">
            <Clock3 size={14} />
            {new Date().toLocaleDateString('ja-JP', {
              month: 'long',
              day: 'numeric',
              weekday: 'short',
            })}
          </span>
        </header>
        <div className="page-content">
          {error && (
            <div role="alert" className="message error">
              <span>{error}</span>
              <button aria-label="エラーを閉じる" onClick={() => setError('')}>
                <X size={16} />
              </button>
            </div>
          )}
          {notice && (
            <div role="status" className="message success">
              <Check size={17} />
              {notice}
            </div>
          )}
          {!view && (
            <div className="empty-state">
              <BookOpen size={36} />
              <h1>{error ? '単語帳を開けませんでした' : '単語帳を開いています'}</h1>
              <p>デスクトップアプリからご利用ください。</p>
              <button className="secondary" onClick={() => void refresh().catch(reportError)}>
                再試行
              </button>
            </div>
          )}
          {view && page !== 'settings' && (
            <div className="page-heading">
              <div>
                <p className="eyebrow">
                  {page === 'home'
                    ? 'A LITTLE, EVERY DAY'
                    : page === 'cards'
                      ? 'YOUR COLLECTION'
                      : 'UNDERSTAND YOUR MEMORY'}
                </p>
                <h1>
                  {page === 'home'
                    ? '今日も、少しずつ。'
                    : page === 'cards'
                      ? 'カード一覧'
                      : '学習の分析'}
                </h1>
                <p>
                  {page === 'home'
                    ? '思い出すたび、知識が自分のものになる。'
                    : page === 'cards'
                      ? '問題・答え・解説を、ひとつのカードに。'
                      : '学習の積み重ねと、これからの復習を見渡す。'}
                </p>
              </div>
              <button
                className="secondary"
                onClick={
                  deck
                    ? openImport
                    : () => {
                        setName('');
                        setModal('create');
                      }
                }
              >
                <Upload size={17} />
                CSVを取り込む
              </button>
            </div>
          )}
          {view && !deck && page !== 'settings' && (
            <section className="welcome panel">
              <div className="welcome-icon">
                <FileSpreadsheet size={42} />
              </div>
              <h2>いつものCSVから、学びを始めよう。</h2>
              <p>
                列を選んで、問題・答え・解説を割り当てるだけ。
                <br />
                復習のタイミングは、学習履歴に合わせて調整されます。
              </p>
              <button
                className="primary"
                onClick={() => {
                  setName('');
                  setModal('create');
                }}
              >
                <Plus size={18} />
                最初の単語帳を作る
              </button>
              <div className="welcome-notes">
                <span>
                  <Check size={14} />
                  端末内で完結
                </span>
                <span>
                  <Check size={14} />
                  CSVの列を自由に選択
                </span>
                <span>
                  <Check size={14} />
                  FSRSで復習
                </span>
              </div>
            </section>
          )}
          {view && deck && queue && page === 'home' && (
            <>
              <div className="stats-grid">
                <Stat
                  label="今日の復習"
                  value={queue.dueReviews}
                  unit="枚"
                  note="復習時期を迎えたカード"
                  color="green"
                />
                <Stat
                  label="これから覚える"
                  value={Math.min(queue.remainingNew, Math.max(0, queue.newLimit - queue.newUsed))}
                  unit="枚"
                  note={`今日の新規上限 ${queue.newLimit}枚`}
                />
                <Stat
                  label="今日の積み重ね"
                  value={queue.newUsed + queue.reviewUsed}
                  unit="枚"
                  note="同じカードは1枚として集計"
                />
                <Stat
                  label="目標保持率"
                  value={Math.round(deck.retention * 1000) / 10}
                  unit="%"
                  note="復習日に思い出したい割合"
                />
              </div>
              <section className="study-banner">
                <div>
                  <span className="pill dark-pill">
                    <span className="status-dot" />
                    今日の学習
                  </span>
                  <h2>
                    {queue.cardIds.length
                      ? 'ひとつ思い出す、その先へ。'
                      : '今日のペースを、大切に。'}
                  </h2>
                  <p>
                    {queue.cardIds.length
                      ? `${deck.name}から ${queue.cardIds.length}枚のカードを用意しました。`
                      : cards.length
                        ? '今の設定で学習できるカードはありません。次の復習までひと休み。'
                        : 'CSVを取り込むと、ここから学習を始められます。'}
                  </p>
                  <div className="banner-actions">
                    <button
                      className="light-button"
                      disabled={!queue.cardIds.length}
                      onClick={() => {
                        setRevealed(false);
                        setModal('study');
                      }}
                    >
                      学習をはじめる
                      <ArrowRight size={18} />
                    </button>
                    <button
                      className="text-on-dark"
                      onClick={() => {
                        setNewBonus(10);
                        setReviewBonus(0);
                        setModal('bonus');
                      }}
                    >
                      今日だけ上乗せ
                      <Plus size={16} />
                    </button>
                  </div>
                </div>
                <div className="book-art" aria-hidden="true">
                  <div className="paper back-paper" />
                  <div className="paper front-paper">
                    <BookOpen size={45} strokeWidth={1.2} />
                    <div />
                    <div />
                    <small>RECALL. REPEAT.</small>
                  </div>
                  <span className="art-star">✧</span>
                </div>
              </section>
              <div className="home-bottom">
                <section className="panel collection-panel">
                  <div className="section-heading">
                    <h2>この単語帳</h2>
                    <button className="quiet" onClick={openDeckSettings}>
                      <Settings2 size={16} />
                      学習設定
                    </button>
                  </div>
                  <div className="deck-summary">
                    <span className="deck-tile">
                      <BookOpen size={26} />
                    </span>
                    <div>
                      <h3>{deck.name}</h3>
                      <p>
                        {cards.length}枚のカード · {cards.filter((c) => c.schedule.reps > 0).length}
                        枚を学習済み
                      </p>
                    </div>
                    <button
                      className="icon-button"
                      aria-label="カード一覧を開く"
                      onClick={() => setPage('cards')}
                    >
                      <ArrowUpRight size={22} />
                    </button>
                  </div>
                  <div
                    className="learning-progress"
                    role="progressbar"
                    aria-label="学習済みカードの割合"
                    aria-valuenow={cards.filter((c) => c.schedule.reps > 0).length}
                    aria-valuemin={0}
                    aria-valuemax={Math.max(cards.length, 1)}
                  >
                    <span
                      style={{
                        width: `${cards.length ? (cards.filter((c) => c.schedule.reps > 0).length / cards.length) * 100 : 0}%`,
                      }}
                    />
                  </div>
                  <div className="legend">
                    <span>
                      <i />
                      学習済み
                    </span>
                    <span>
                      {cards.length
                        ? Math.round(
                            (cards.filter((c) => c.schedule.reps > 0).length / cards.length) * 100,
                          )
                        : 0}
                      %
                    </span>
                  </div>
                </section>
                <section className="panel tip-panel">
                  <span className="tip-icon">
                    <Sparkles size={20} />
                  </span>
                  <p className="eyebrow">学びのヒント</p>
                  <h3>
                    答えを見る前に、
                    <br />
                    一度、思い出してみる。
                  </h3>
                  <p>
                    「難しい」は、思い出せたときに。
                    <br />
                    思い出せなかったら「忘れた」を選びましょう。
                  </p>
                </section>
              </div>
              <p className="footnote">
                <ShieldCheck size={14} />
                学習内容・履歴・分析は、この端末内で処理します。
              </p>
            </>
          )}
          {view && deck && page === 'cards' && (
            <section className="panel cards-panel">
              <div className="section-heading">
                <h2>
                  {deck.name}
                  <span className="count">{cards.length}枚</span>
                </h2>
                <label className="search-box">
                  <Search size={16} />
                  <input
                    aria-label="カードを検索"
                    placeholder="問題・答え・選択肢から検索"
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                  />
                </label>
              </div>
              <div className="table-scroll">
                <table>
                  <thead>
                    <tr>
                      <th>問題</th>
                      <th>答え</th>
                      <th>次の復習</th>
                      <th>学習回数</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {cards
                      .filter((c) =>
                        `${c.question}\n${c.answer}\n${c.choices.map((choice) => choice.text).join('\n')}`
                          .toLocaleLowerCase()
                          .includes(query.toLocaleLowerCase()),
                      )
                      .map((c) => (
                        <tr key={c.id}>
                          <td className="text-cell">
                            {c.question}
                            {c.choices.length > 0 && (
                              <small className="muted card-choice-count">
                                選択肢 {c.choices.length}件
                              </small>
                            )}
                          </td>
                          <td className="text-cell muted">{c.answer}</td>
                          <td>
                            <span className={`pill ${c.schedule.reps ? '' : 'neutral'}`}>
                              {date(c.schedule.due)}
                            </span>
                          </td>
                          <td>{c.schedule.reps}回</td>
                          <td>
                            <button
                              className="quiet"
                              onClick={() => {
                                setEdit({ ...c });
                                setModal('edit');
                              }}
                            >
                              編集
                            </button>
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
              {!cards.length && (
                <div className="chart-empty">CSVを取り込んでカードを追加しましょう。</div>
              )}
            </section>
          )}
          {view && deck && page === 'analytics' && (
            <>
              <div className="stats-grid">
                <Stat
                  label="学習済み"
                  value={analytics?.learnedCards ?? 0}
                  unit="枚"
                  note={`${cards.length}枚のうち`}
                />
                <Stat
                  label="これまでの自己評価"
                  value={analytics?.totalReviews ?? 0}
                  unit="回"
                  note="取り消した評価を除く"
                />
                <Stat
                  label="思い出せた割合"
                  value={
                    analytics?.successRate == null ? '—' : Math.round(analytics.successRate * 100)
                  }
                  unit={analytics?.successRate == null ? '' : '%'}
                  note="難しい・普通・簡単の割合"
                />
                <Stat
                  label="目標保持率"
                  value={Math.round(deck.retention * 1000) / 10}
                  unit="%"
                  note="学習設定から変更できます"
                />
              </div>
              <section className="panel">
                <div className="section-heading">
                  <div>
                    <h2>これから30日間の復習</h2>
                    <p>通常の枚数設定で学習を続ける場合のシミュレーション</p>
                  </div>
                  <span className="pill neutral">推定</span>
                </div>
                <Forecast analytics={analytics} />
                <p className="footnote">
                  通常の学習上限・FSRS設定と既定の回答傾向に基づく予測です。当日上乗せは含めません。実際の回答によって変わります。
                </p>
              </section>
              <section className="panel curve-panel">
                <div className="section-heading">
                  <h2>カードごとの推定忘却曲線</h2>
                  <select
                    aria-label="忘却曲線のカード"
                    value={curveCard}
                    onChange={(e) => setCurveCard(e.target.value)}
                  >
                    <option value="">カードを選択</option>
                    {cards.map((c) => (
                      <option value={c.id} key={c.id}>
                        {c.question.slice(0, 60)}
                      </option>
                    ))}
                  </select>
                </div>
                <Curve points={curve} />
                <p className="footnote">
                  復習せずに時間が経過した場合の推定です。記憶力の実測値や保証ではありません。
                </p>
              </section>
              <section className="panel optimize-panel">
                <div>
                  <span className="eyebrow">PERSONALIZE</span>
                  <h2>あなたの学習履歴で調整</h2>
                  <p>
                    端末内でFSRSパラメータを計算します。
                    <br />
                    翌日以降の復習履歴：{analytics?.optimizationSamples ?? 0}件 / 調整の目安512件
                  </p>
                  {!analytics?.canOptimize && (
                    <p className="muted">履歴が少ない間は、既定のパラメータを使います。</p>
                  )}
                  {optimization?.status !== 'idle' && optimization?.message && (
                    <p role="status">{optimization.message}</p>
                  )}
                  {optimization?.status === 'running' && (
                    <progress
                      aria-label="個人向け調整の進捗"
                      max={optimization.total || 1}
                      value={optimization.current}
                    />
                  )}
                </div>
                <div className="stack-actions">
                  {optimization?.status === 'running' ? (
                    <button
                      className="secondary"
                      onClick={() =>
                        void run(async () => {
                          await transport.command({ type: 'cancelOptimization' });
                          await loadAnalytics();
                        })
                      }
                    >
                      計算を中断
                    </button>
                  ) : (
                    <button
                      className="primary"
                      disabled={!analytics?.canOptimize || busy}
                      onClick={() =>
                        void run(async () => {
                          await transport.command({ type: 'startOptimization', deckId: deck.id });
                          await loadAnalytics();
                        })
                      }
                    >
                      <Sparkles size={16} />
                      端末内で計算する
                    </button>
                  )}
                  {optimization?.status === 'ready' && optimization.deckId === deck.id && (
                    <button
                      className="secondary"
                      onClick={() =>
                        void run(async () => {
                          await transport.command({ type: 'applyOptimization' });
                          await refresh();
                          await loadAnalytics();
                          setNotice('今後の復習に使うパラメータを更新しました。');
                        })
                      }
                    >
                      計算結果を適用
                    </button>
                  )}
                </div>
              </section>
            </>
          )}
          {view && page === 'settings' && (
            <>
              <div className="page-heading">
                <div>
                  <p className="eyebrow">MAKE IT YOURS</p>
                  <h1>設定とバックアップ</h1>
                  <p>自分のペースと、学習データの引き継ぎ。</p>
                </div>
              </div>
              <section className="panel settings-panel">
                <h2>学習日の切り替え</h2>
                <p>「今日だけ上乗せ」は、ここで設定した時刻に終了します。</p>
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    void mutate(
                      { type: 'settings', settings: { timezone, dayStartHour: dayStart } },
                      '学習日の設定を保存しました。',
                    );
                  }}
                >
                  <div className="form-row">
                    <label>
                      開始時刻
                      <select
                        value={dayStart}
                        onChange={(e) => setDayStart(Number(e.target.value))}
                      >
                        {Array.from({ length: 24 }, (_, i) => (
                          <option key={i} value={i}>
                            {i}:00
                          </option>
                        ))}
                      </select>
                    </label>
                    <label>
                      学習用タイムゾーン
                      <input
                        required
                        value={timezone}
                        onChange={(e) => setTimezone(e.target.value)}
                        placeholder="Asia/Tokyo"
                      />
                    </label>
                  </div>
                  <p className="footnote">
                    初回は端末のタイムゾーンを使います。バックアップから復元しても、この設定を引き継ぎます。
                  </p>
                  <button className="primary" disabled={busy}>
                    設定を保存
                  </button>
                </form>
              </section>
              <section className="panel settings-panel">
                <div className="section-heading">
                  <div>
                    <h2>暗号化バックアップ</h2>
                    <p>全単語帳・学習履歴・学習設定をまとめて保存・復元します。</p>
                  </div>
                  <LockKeyhole size={28} className="green-text" />
                </div>
                <div className="backup-actions">
                  <button
                    className="secondary"
                    onClick={() => {
                      setPassphrase('');
                      setPassConfirm('');
                      setModal('backup');
                    }}
                  >
                    <Download size={18} />
                    バックアップを作成
                  </button>
                  <button
                    className="secondary"
                    onClick={() => {
                      setPassphrase('');
                      setRestore(null);
                      setModal('restore');
                    }}
                  >
                    <RotateCcw size={18} />
                    バックアップから復元
                  </button>
                </div>
                <p className="footnote">パスフレーズを失うと、そのバックアップは復元できません。</p>
              </section>
              <section className="panel privacy-panel">
                <ShieldCheck size={28} />
                <div>
                  <h2>この端末で完結する学習</h2>
                  <p>
                    取り込み・学習・分析は端末内で処理します。通信や自動更新は行いません。
                    <br />
                    端末内のデータ保護は、OSの暗号化とログイン保護に委ねています。
                  </p>
                  <p className="footnote">Kotoba 0.2.0 · FSRS-6 · 更新はインストーラで手動適用</p>
                </div>
              </section>
            </>
          )}
        </div>
      </main>
      {modal === 'create' && (
        <Modal title="単語帳を作成" close={close}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void run(async () => {
                const id = await transport.command<string>({ type: 'createDeck', name });
                setSelected(id);
                await refresh();
                setPage('home');
                openImport();
              });
            }}
          >
            <p className="modal-description">覚えたいテーマに、名前をつけましょう。</p>
            <label>
              単語帳の名前
              <input
                autoFocus
                required
                maxLength={200}
                value={name}
                placeholder="例：仕事の用語、英単語"
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <div className="modal-actions">
              <button type="button" className="secondary" onClick={close}>
                キャンセル
              </button>
              <button className="primary" disabled={busy}>
                作成してCSVを選ぶ
                <ArrowRight size={16} />
              </button>
            </div>
          </form>
        </Modal>
      )}
      {modal === 'import' && deck && (
        <Modal title={`CSVを取り込む · ${deck.name}`} close={close} wide>
          <div className="steps">
            <span className={!source ? 'current' : ''}>1 ファイルを選ぶ</span>
            <ChevronRight size={15} />
            <span className={source && !preview ? 'current' : ''}>2 列を割り当てる</span>
            <ChevronRight size={15} />
            <span className={preview ? 'current' : ''}>3 内容を確認</span>
          </div>
          {!source && (
            <div className="file-picker">
              <FileSpreadsheet size={44} />
              <h3>CSVファイルを選択</h3>
              <p>先頭行を見出しとして読み込みます。</p>
              <label>
                文字コード
                <select value={encoding} onChange={(e) => setEncoding(e.target.value)}>
                  <option value="utf-8">UTF-8 / UTF-8 BOM</option>
                  <option value="cp932">CP932（Windows）</option>
                </select>
              </label>
              <button className="primary" disabled={busy} onClick={() => void chooseCsv()}>
                <Upload size={17} />
                ファイルを選ぶ
              </button>
            </div>
          )}
          {source && !preview && (
            <>
              <p className="modal-description">
                {source.rows.length}行を読み込みました。使う列を選択してください。
              </p>
              <div className="mapping-grid">
                {(
                  [
                    ['question', '問題', false],
                    ['answer', '答え', false],
                    ['explanation', '解説', true],
                    ['id', '照合用ID', true],
                  ] as const
                ).map(([field, label, optional]) => (
                  <label key={field}>
                    {label}
                    {optional && <small>任意</small>}
                    <select
                      value={mapping[field] ?? ''}
                      onChange={(e) => {
                        const column = e.target.value === '' ? null : Number(e.target.value);
                        setMapping({
                          ...mapping,
                          [field]: column,
                          choices: mapping.choices.filter((choice) => choice !== column),
                        });
                      }}
                    >
                      {optional && <option value="">使用しない</option>}
                      {source.headers.map((h, i) => (
                        <option key={i} value={i}>
                          {i + 1}. {h || '見出しなし'}
                        </option>
                      ))}
                    </select>
                  </label>
                ))}
              </div>
              <fieldset className="choice-columns">
                <legend>
                  選択肢の列 <small>任意・複数選択可</small>
                </legend>
                <div className="choice-column-options">
                  {source.headers.map((header, column) => (
                    <label key={column}>
                      <input
                        type="checkbox"
                        checked={mapping.choices.includes(column)}
                        disabled={[
                          mapping.question,
                          mapping.answer,
                          mapping.explanation,
                          mapping.id,
                        ].includes(column)}
                        onChange={(e) =>
                          setMapping({
                            ...mapping,
                            choices: e.target.checked
                              ? [...mapping.choices, column].sort((a, b) => a - b)
                              : mapping.choices.filter((choice) => choice !== column),
                          })
                        }
                      />
                      {column + 1}. {header || '見出しなし'}
                    </label>
                  ))}
                </div>
                {mapping.choices.length > 0 && (
                  <>
                    <label>
                      選択肢セルの読み方
                      <select
                        value={
                          mapping.choiceSeparator === null
                            ? 'whole'
                            : mapping.choiceSeparator === '\n'
                              ? 'lines'
                              : 'custom'
                        }
                        onChange={(e) =>
                          setMapping({
                            ...mapping,
                            choiceSeparator:
                              e.target.value === 'whole'
                                ? null
                                : e.target.value === 'lines'
                                  ? '\n'
                                  : '',
                          })
                        }
                      >
                        <option value="whole">セル全体をそのまま表示</option>
                        <option value="lines">改行で選択肢を分ける</option>
                        <option value="custom">区切り文字を指定する</option>
                      </select>
                    </label>
                    {mapping.choiceSeparator !== null && mapping.choiceSeparator !== '\n' && (
                      <label>
                        選択肢の区切り文字
                        <input
                          value={mapping.choiceSeparator}
                          placeholder="例：| または ;"
                          onChange={(e) =>
                            setMapping({
                              ...mapping,
                              choiceSeparator: e.target.value,
                            })
                          }
                        />
                      </label>
                    )}
                  </>
                )}
                <p className="footnote">
                  1列にまとめた選択肢も取り込めます。「そのまま表示」は改行やカンマを保ち、空のセルは省きます。
                </p>
              </fieldset>
              <p className="footnote">
                IDを使わない場合は同じ問題文の行を1枚にまとめます。IDを使う場合は同じID・問題文の行をまとめます。
                答え・解説・選択肢は重複する内容を除いて保持し、プレビューで確認できます。選択しない列は保存しません。
              </p>
              <div className="table-scroll csv-table">
                <table>
                  <thead>
                    <tr>
                      {source.headers.map((h, i) => (
                        <th key={i}>{h}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {source.rows.slice(0, 12).map((row, i) => (
                      <tr key={i}>
                        {row.map((cell, j) => (
                          <td key={j} className="text-cell">
                            {cell}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="modal-actions">
                <span className="muted">先頭12行を表示</span>
                <button
                  className="primary"
                  disabled={busy}
                  onClick={() =>
                    void run(async () =>
                      setPreview(
                        await transport.command<Preview>({
                          type: 'previewImport',
                          deckId: deck.id,
                          sourceToken: source.token,
                          mapping,
                        }),
                      ),
                    )
                  }
                >
                  更新内容をプレビュー
                  <ArrowRight size={16} />
                </button>
              </div>
            </>
          )}
          {preview && (
            <>
              <div className="preview-counts">
                {[
                  ['new', '追加'],
                  ['update', '更新'],
                  ['unchanged', '変更なし'],
                ].map(([kind, label]) => (
                  <span className="pill" key={kind}>
                    {label} {preview.changes.filter((c) => c.kind === kind).length}枚
                  </span>
                ))}
                <span className="pill neutral">CSVにない既存カード {preview.missing}枚を保持</span>
                {preview.mergedRows > 0 && (
                  <span className="pill">重複 {preview.mergedRows}行を統合</span>
                )}
              </div>
              {preview.errors.length > 0 && (
                <div role="alert" className="issues">
                  <strong>適用前に確認してください</strong>
                  {preview.errors.map((e, i) => (
                    <p key={i}>
                      {e.line}行目：{e.message}
                    </p>
                  ))}
                </div>
              )}
              <div className="changes-list">
                {preview.changes.map((c) => (
                  <article key={c.line} className="change">
                    <div>
                      <span className={`pill ${c.kind === 'unchanged' ? 'neutral' : ''}`}>
                        {c.kind === 'new' ? '追加' : c.kind === 'update' ? '更新' : '変更なし'}
                      </span>
                      <small>{c.sourceLines.join('・')}行目</small>
                      {c.sourceLines.length > 1 && <small>{c.sourceLines.length}行 → 1枚</small>}
                    </div>
                    {(['question', 'answer', 'explanation'] as const).map((field, i) => (
                      <div key={field} className="change-field">
                        <small>{['問題', '答え', '解説'][i]}</small>
                        {c.before && c.before[field] !== c.after[field] && (
                          <del>{c.before[field] || '（空欄）'}</del>
                        )}
                        <p>{c.after[field] || '（空欄）'}</p>
                      </div>
                    ))}
                    {(c.after.choices.length > 0 || (c.before?.choices.length ?? 0) > 0) && (
                      <div className="change-field change-choices">
                        <small>選択肢</small>
                        {c.before &&
                          JSON.stringify(c.before.choices) !== JSON.stringify(c.after.choices) && (
                            <div className="previous-choices">
                              <small>変更前</small>
                              <Choices choices={c.before.choices} />
                            </div>
                          )}
                        {c.after.choices.length ? (
                          <Choices choices={c.after.choices} />
                        ) : (
                          <p>（なし）</p>
                        )}
                      </div>
                    )}
                    {c.notes.length > 0 && (
                      <div className="merge-notes">
                        {c.notes.map((note, i) => (
                          <p key={i}>{note}</p>
                        ))}
                      </div>
                    )}
                  </article>
                ))}
              </div>
              <div className="modal-actions">
                <button className="secondary" onClick={() => setPreview(null)}>
                  <ArrowLeft size={16} />
                  列の選択に戻る
                </button>
                <button
                  className="primary"
                  disabled={busy || preview.errors.length > 0}
                  onClick={() =>
                    void run(async () => {
                      await transport.command({ type: 'applyImport', token: preview.token });
                      setSource(null);
                      setPreview(null);
                      setModal(null);
                      await refresh();
                      setNotice('CSVを取り込みました。同じカードの学習履歴は保持されています。');
                    })
                  }
                >
                  内容を適用する
                  <Check size={16} />
                </button>
              </div>
            </>
          )}
        </Modal>
      )}
      {modal === 'study' && (
        <Modal title={deck?.name ?? '学習'} close={close} wide>
          {currentCard ? (
            <>
              <div className="study-meta">
                <span className="pill neutral">{currentCard.schedule.reps ? '復習' : '新規'}</span>
                <span>残り {queue?.cardIds.length}枚</span>
              </div>
              <section className="flashcard">
                <span className="eyebrow">QUESTION</span>
                <h2 className="plain-text">{currentCard.question}</h2>
                <Choices choices={currentCard.choices} />
                {revealed && (
                  <div className="answer-area">
                    <span className="eyebrow">ANSWER</span>
                    <p className="answer-text plain-text">{currentCard.answer}</p>
                    {currentCard.explanation && (
                      <div className="explanation">
                        <span>解説</span>
                        <p className="plain-text">{currentCard.explanation}</p>
                      </div>
                    )}
                  </div>
                )}
              </section>
              {revealed ? (
                <>
                  <p className="rating-hint">
                    どのくらい思い出せましたか？ 「難しい」は思い出せた場合に選びます。
                  </p>
                  <div className="ratings">
                    {['忘れた', '難しい', '普通', '簡単'].map((label, i) => (
                      <button
                        key={label}
                        className={`rating rating-${i}`}
                        disabled={busy}
                        onClick={() => grade(i + 1)}
                      >
                        <span>{label}</span>
                        <kbd>{i + 1}</kbd>
                      </button>
                    ))}
                  </div>
                </>
              ) : (
                <button
                  className="primary reveal-button"
                  disabled={busy}
                  onClick={() => setRevealed(true)}
                >
                  答えを見る<kbd>Space</kbd>
                </button>
              )}
            </>
          ) : (
            <div className="study-done">
              <span className="done-icon">
                <Check size={34} />
              </span>
              <h2>{nextRepetition ? '少し待って、もう一度' : '今日の学習が終わりました'}</h2>
              <p>
                {nextRepetition
                  ? '忘れたカードを1分後にもう一度出題します。この画面で待つか、あとで戻ってきてください。'
                  : '積み重ねを保存しました。次の復習でまた会いましょう。'}
              </p>
              <button className="primary" onClick={close}>
                ホームに戻る
              </button>
            </div>
          )}
          <div className="study-footer">
            <button
              className="quiet"
              disabled={!view?.canUndo || busy}
              onClick={() => void mutate({ type: 'undo' }, undefined, true)}
            >
              <RotateCcw size={15} />
              直前の自己評価を取り消す
            </button>
            <small>学習日は {view?.settings.dayStartHour}:00 に切り替わります</small>
          </div>
        </Modal>
      )}
      {modal === 'bonus' && deck && (
        <Modal title="今日だけ上乗せ" close={close}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void mutate(
                { type: 'addBonus', deckId: deck.id, new: newBonus, review: reviewBonus },
                '今日の学習上限に追加しました。',
              );
            }}
          >
            <p className="modal-description">
              通常の枚数設定は変えず、この学習日だけ追加します。次の{view?.settings.dayStartHour}
              :00に終了します。
            </p>
            <label>
              今日の新規に追加する枚数
              <input
                type="number"
                min="0"
                max="1000000"
                required
                value={newBonus}
                onChange={(e) => setNewBonus(Number(e.target.value))}
              />
            </label>
            <label>
              今日の復習に追加する枚数
              <input
                type="number"
                min="0"
                max="1000000"
                required
                disabled={deck.reviewLimit === null}
                value={reviewBonus}
                onChange={(e) => setReviewBonus(Number(e.target.value))}
              />
            </label>
            {deck.reviewLimit === null && (
              <p className="footnote">復習の通常上限は「上限なし」です。</p>
            )}
            <p className="footnote">復習予定日は前倒しされません。</p>
            <div className="modal-actions">
              <button type="button" className="secondary" onClick={close}>
                キャンセル
              </button>
              <button className="primary" disabled={busy}>
                上乗せする
                <Plus size={16} />
              </button>
            </div>
          </form>
        </Modal>
      )}
      {modal === 'deckSettings' && deck && (
        <Modal title="学習設定" close={close}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void mutate(
                {
                  type: 'configureDeck',
                  deckId: deck.id,
                  retention: retention / 100,
                  newLimit,
                  reviewLimit: reviewLimit === '' ? null : Number(reviewLimit),
                },
                '学習設定を保存しました。',
              );
            }}
          >
            <label>
              目標保持率（%）
              <input
                type="number"
                min="1"
                max="99.9"
                step="0.1"
                required
                value={retention}
                onChange={(e) => setRetention(Number(e.target.value))}
              />
            </label>
            <label>
              通常の新規枚数 / 日
              <input
                type="number"
                min="0"
                max="1000000"
                required
                value={newLimit}
                onChange={(e) => setNewLimit(Number(e.target.value))}
              />
            </label>
            <label>
              通常の復習枚数 / 日
              <input
                type="number"
                min="0"
                max="1000000"
                placeholder="上限なし"
                value={reviewLimit}
                onChange={(e) => setReviewLimit(e.target.value)}
              />
            </label>
            <p className="footnote">
              復習枚数を空欄にすると上限なしになります。上限を超えた復習も残件として表示します。
            </p>
            <div className="modal-actions">
              <button className="primary" disabled={busy}>
                保存する
              </button>
            </div>
          </form>
        </Modal>
      )}
      {modal === 'edit' && edit && (
        <Modal title="カードを編集" close={close}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void mutate(
                {
                  type: 'editCard',
                  cardId: edit.id,
                  question: edit.question,
                  answer: edit.answer,
                  explanation: edit.explanation,
                  choices: edit.choices,
                },
                'カードを更新しました。学習履歴は保持されています。',
              );
            }}
          >
            {(['question', 'answer', 'explanation'] as const).map((field, i) => (
              <label key={field}>
                {['問題', '答え', '解説'][i]}
                <textarea
                  required={field !== 'explanation'}
                  rows={field === 'explanation' ? 4 : 3}
                  value={edit[field]}
                  onChange={(e) => setEdit({ ...edit, [field]: e.target.value })}
                />
              </label>
            ))}
            <fieldset className="choice-editor">
              <legend>選択肢</legend>
              {edit.choices.map((choice, i) => (
                <div className="choice-editor-row" key={i}>
                  <label>
                    選択肢 {i + 1} の見出し
                    <input
                      value={choice.label}
                      onChange={(e) =>
                        setEdit({
                          ...edit,
                          choices: edit.choices.map((value, index) =>
                            index === i ? { ...value, label: e.target.value } : value,
                          ),
                        })
                      }
                    />
                  </label>
                  <label>
                    選択肢 {i + 1} の内容
                    <textarea
                      required
                      rows={2}
                      value={choice.text}
                      onChange={(e) =>
                        setEdit({
                          ...edit,
                          choices: edit.choices.map((value, index) =>
                            index === i ? { ...value, text: e.target.value } : value,
                          ),
                        })
                      }
                    />
                  </label>
                  <button
                    type="button"
                    className="quiet"
                    aria-label={`選択肢 ${i + 1} を削除`}
                    onClick={() =>
                      setEdit({
                        ...edit,
                        choices: edit.choices.filter((_, index) => index !== i),
                      })
                    }
                  >
                    <X size={16} />
                  </button>
                </div>
              ))}
              <button
                type="button"
                className="secondary"
                onClick={() =>
                  setEdit({
                    ...edit,
                    choices: [...edit.choices, { label: '', text: '' }],
                  })
                }
              >
                <Plus size={16} />
                選択肢を追加
              </button>
            </fieldset>
            <div className="modal-actions">
              <button className="primary" disabled={busy}>
                変更を保存
              </button>
            </div>
          </form>
        </Modal>
      )}
      {modal === 'backup' && (
        <Modal title="暗号化バックアップを作成" close={close}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (passphrase !== passConfirm) {
                setError('パスフレーズが一致しません。');
                return;
              }
              void run(async () => {
                try {
                  if (await transport.exportBackup(passphrase)) {
                    setModal(null);
                    setNotice('暗号化バックアップを保存しました。');
                  }
                } finally {
                  setPassphrase('');
                  setPassConfirm('');
                }
              });
            }}
          >
            <p className="modal-description">全単語帳・学習履歴・設定を暗号化して保存します。</p>
            <label>
              パスフレーズ
              <input
                type="password"
                autoComplete="new-password"
                required
                value={passphrase}
                onChange={(e) => setPassphrase(e.target.value)}
              />
            </label>
            <label>
              パスフレーズをもう一度
              <input
                type="password"
                autoComplete="new-password"
                required
                value={passConfirm}
                onChange={(e) => setPassConfirm(e.target.value)}
              />
            </label>
            <p className="footnote">
              長く推測されにくいパスフレーズを使ってください。アプリには保存されません。失うとバックアップは復元できません。
            </p>
            <div className="modal-actions">
              <button className="primary" disabled={busy}>
                {busy ? '暗号化しています…' : '保存先を選んで作成'}
                <LockKeyhole size={16} />
              </button>
            </div>
          </form>
        </Modal>
      )}
      {modal === 'restore' && (
        <Modal title="バックアップから復元" close={close}>
          {restore ? (
            <>
              <p className="modal-description">
                現在の全データを、次の内容で置き換えます。現在の状態は端末内に退避します。
              </p>
              <div className="restore-summary">
                <strong>
                  {restore.decks.length}単語帳 · {restore.cards}枚 · {restore.reviews}回の学習履歴
                </strong>
                {restore.decks.map((d, i) => (
                  <p key={i}>
                    {d.name}
                    <span>{d.cards}枚</span>
                  </p>
                ))}
              </div>
              <div className="modal-actions">
                <button className="secondary" onClick={close}>
                  キャンセル
                </button>
                <button
                  className="primary"
                  disabled={busy}
                  onClick={() =>
                    void run(async () => {
                      await transport.command({ type: 'restore', token: restore.token });
                      setRestore(null);
                      setModal(null);
                      setSelected('');
                      await refresh();
                      setNotice('全体を復元しました。復元前のデータも端末内に退避しています。');
                    })
                  }
                >
                  全体を置き換えて復元
                </button>
              </div>
            </>
          ) : (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void run(async () => {
                  try {
                    setRestore(await transport.previewRestore(passphrase));
                  } finally {
                    setPassphrase('');
                  }
                });
              }}
            >
              <p className="modal-description">
                バックアップを開いてから、復元内容を確認できます。
              </p>
              <label>
                バックアップのパスフレーズ
                <input
                  type="password"
                  autoComplete="off"
                  required
                  value={passphrase}
                  onChange={(e) => setPassphrase(e.target.value)}
                />
              </label>
              <div className="modal-actions">
                <button className="primary" disabled={busy}>
                  {busy ? '復号・検証しています…' : 'ファイルを選んで内容を確認'}
                </button>
              </div>
            </form>
          )}
        </Modal>
      )}
      {error && modal && (
        <div className="modal-error" role="alert">
          {error}
          <button aria-label="メッセージを閉じる" onClick={() => setError('')}>
            <X size={16} />
          </button>
        </div>
      )}
    </div>
  );
}

function Stat({
  label,
  value,
  unit,
  note,
  color = '',
}: {
  label: string;
  value: number | string;
  unit: string;
  note: string;
  color?: string;
}) {
  return (
    <section className={`stat panel ${color}`}>
      <p>{label}</p>
      <div className="stat-value">
        {value}
        <span>{unit}</span>
      </div>
      <small>{note}</small>
    </section>
  );
}
function Forecast({ analytics }: { analytics: Analytics | null }) {
  if (!analytics) return <div className="chart-empty">端末内で予測を計算しています…</div>;
  const max = Math.max(1, ...analytics.forecast.map((p) => p.reviews + p.newCards));
  return (
    <>
      <svg
        viewBox="0 0 720 205"
        className="chart"
        role="img"
        aria-label="30日分の新規学習と復習枚数の予測"
      >
        <line x1="30" x2="710" y1="170" y2="170" className="grid-line" />
        {analytics.forecast.map((p) => (
          <g key={p.day}>
            <rect
              x={35 + p.day * 22.4}
              y={170 - ((p.reviews + p.newCards) / max) * 140}
              width="13"
              height={(p.newCards / max) * 140}
              fill="#bfcec5"
              rx="2"
            />
            <rect
              x={35 + p.day * 22.4}
              y={170 - (p.reviews / max) * 140}
              width="13"
              height={(p.reviews / max) * 140}
              fill="#358068"
              rx="2"
            />
            <title>
              {p.day === 0 ? '今日' : `${p.day}日後`}：復習{p.reviews}枚、新規{p.newCards}枚
            </title>
          </g>
        ))}
        <text x="0" y="36">
          {max}
        </text>
        <text x="12" y="175">
          0
        </text>
        <text x="35" y="198">
          今日
        </text>
        <text x="352" y="198">
          15日後
        </text>
        <text x="654" y="198">
          29日後
        </text>
      </svg>
      <div className="chart-legend">
        <span>
          <i />
          復習
        </span>
        <span>
          <i className="light" />
          新規
        </span>
      </div>
    </>
  );
}
