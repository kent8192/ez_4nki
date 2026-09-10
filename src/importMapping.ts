import type { Mapping } from './types';

export function suggestMapping(headers: string[], previous: Mapping | null): Mapping {
  if (headers.length < 2) throw new Error('CSVには問題と答えの2列以上が必要です。');
  const find = (names: string[]) => {
    const index = headers.findIndex((header) => names.includes(header.trim().toLowerCase()));
    return index < 0 ? null : index;
  };
  let question = find(['問題', 'question', 'front']);
  let answer = find(['答え', 'answer', 'back']);
  const idHint = find(['id']);
  const explanationHint = find(['解説', 'explanation']);
  const used = new Set([question, answer].filter((column): column is number => column !== null));
  const required = Number(question === null) + Number(answer === null);
  const optional = (column: number | null) => {
    if (column === null || used.has(column) || headers.length - used.size - 1 < required)
      return null;
    used.add(column);
    return column;
  };
  // Keep the deck's explicit question-matching choice, including when an ID header exists.
  const id = optional(previous?.id === null ? null : idHint);
  const explanation = optional(explanationHint);
  const fallback = () => {
    const remaining = headers.flatMap((_, column) => (used.has(column) ? [] : [column]));
    const column =
      remaining.find((column) => column !== idHint && column !== explanationHint) ?? remaining[0];
    used.add(column);
    return column;
  };
  question ??= fallback();
  answer ??= fallback();
  const choices =
    previous?.choices.length === 0
      ? []
      : headers.flatMap((header, column) =>
          !used.has(column) &&
          ['選択肢', 'choices', 'options'].includes(header.trim().toLowerCase())
            ? [column]
            : [],
        );
  return {
    question,
    answer,
    explanation,
    id,
    choices,
    choiceSeparator: choices.length ? (previous?.choiceSeparator ?? null) : null,
  };
}
