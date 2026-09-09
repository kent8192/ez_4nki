export interface Settings {
  timezone: string;
  dayStartHour: number;
}
export interface Mapping {
  question: number;
  answer: number;
  explanation: number | null;
  id: number | null;
}
export interface Schedule {
  stability: number | null;
  difficulty: number | null;
  lastReview: number | null;
  due: number | null;
  reps: number;
  lapses: number;
}
export interface Card {
  id: string;
  deckId: string;
  sourceId: string | null;
  question: string;
  answer: string;
  explanation: string;
  createdAt: number;
  schedule: Schedule;
}
export interface Deck {
  id: string;
  name: string;
  createdAt: number;
  retention: number;
  newLimit: number;
  reviewLimit: number | null;
  parameters: number[];
  parameterVersion: string;
  mapping: Mapping | null;
}
export interface Queue {
  day: string;
  cardIds: string[];
  remainingNew: number;
  dueReviews: number;
  newUsed: number;
  reviewUsed: number;
  newLimit: number;
  reviewLimit: number | null;
  newBonus: number;
  reviewBonus: number;
}
export interface View {
  revision: number;
  settings: Settings;
  decks: Deck[];
  cards: Card[];
  queues: Record<string, Queue>;
  canUndo: boolean;
  reviewCount: number;
}
export interface Change {
  line: number;
  kind: 'new' | 'update' | 'unchanged';
  before: Card | null;
  after: Card;
}
export interface Preview {
  token: string;
  deckId: string;
  changes: Change[];
  errors: { line: number; message: string }[];
  missing: number;
}
export interface Source {
  token: string;
  headers: string[];
  rows: string[][];
}
export interface Analytics {
  totalReviews: number;
  successRate: number | null;
  learnedCards: number;
  forecast: { day: number; reviews: number; newCards: number }[];
  optimizationSamples: number;
  canOptimize: boolean;
}
export interface CurvePoint {
  days: number;
  retention: number;
}
export interface Optimization {
  status: string;
  current: number;
  total: number;
  message: string;
  deckId: string | null;
}
export interface RestorePreview {
  token: string;
  decks: { name: string; cards: number }[];
  cards: number;
  reviews: number;
}
export interface Transport {
  command<T>(request: Record<string, unknown>): Promise<T>;
  pickCsv(encoding: string): Promise<Source | null>;
  exportBackup(passphrase: string): Promise<boolean>;
  previewRestore(passphrase: string): Promise<RestorePreview | null>;
}
