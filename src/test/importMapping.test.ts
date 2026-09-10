import { describe, expect, it } from 'vitest';
import { suggestMapping } from '../importMapping';
import type { Mapping } from '../types';

function permutations<T>(values: T[]): T[][] {
  return values.length === 0
    ? [[]]
    : values.flatMap((value, index) =>
        permutations(values.filter((_, i) => i !== index)).map((rest) => [value, ...rest]),
      );
}

const previous: Mapping = {
  question: 1,
  answer: 2,
  explanation: 3,
  id: 0,
  choices: [4],
  choiceSeparator: '|',
};

describe('column suggestions', () => {
  it('finds named columns across all 720 permutations without reusing saved positions', () => {
    for (const headers of permutations(['ID', '問題', '答え', '解説', '選択肢', '未使用'])) {
      for (const saved of [
        null,
        previous,
        { ...previous, id: null },
        { ...previous, choices: [] },
      ]) {
        const mapping = suggestMapping(headers, saved);
        expect(headers[mapping.question]).toBe('問題');
        expect(headers[mapping.answer]).toBe('答え');
        expect(headers[mapping.explanation!]).toBe('解説');
        if (saved?.id === null) expect(mapping.id).toBeNull();
        else expect(headers[mapping.id!]).toBe('ID');
        if (saved?.choices.length === 0) expect(mapping.choices).toEqual([]);
        else expect(mapping.choices.map((column) => headers[column])).toEqual(['選択肢']);
        const columns = [
          mapping.question,
          mapping.answer,
          mapping.id,
          mapping.explanation,
          ...mapping.choices,
        ].filter((column) => column !== null);
        expect(new Set(columns).size).toBe(columns.length);
      }
    }
  });

  it.each(
    [
      ['ID', 'term', 'definition'],
      ['ID', 'question', 'translation'],
      ['ID', 'unknown', 'answer'],
      ['explanation', 'unknown', 'unrecognized', 'ID'],
      ['ID', 'answer'],
      ['question', 'question'],
      ['ID', 'ID', 'question', 'answer'],
      ['ID', 'explanation'],
      ['', '', 'choices'],
    ].map((headers) => ({ headers })),
  )('reserves distinct required columns for $headers', ({ headers }) => {
    for (const ordered of permutations(headers)) {
      const mapping = suggestMapping(ordered, null);
      const columns = [
        mapping.question,
        mapping.answer,
        mapping.id,
        mapping.explanation,
        ...mapping.choices,
      ].filter((column): column is number => column !== null);
      expect(new Set(columns).size).toBe(columns.length);
      expect(
        columns.every(
          (column) => Number.isInteger(column) && column >= 0 && column < headers.length,
        ),
      ).toBe(true);
    }
  });

  it('normalizes header spelling without inspecting or normalizing card IDs', () => {
    expect(suggestMapping(['  iD ', ' QUESTION ', ' Answer ', 'Options'], null)).toEqual({
      question: 1,
      answer: 2,
      id: 0,
      explanation: null,
      choices: [3],
      choiceSeparator: null,
    });
  });

  it('keeps the previous choice delimiter and can omit choices when that deck does not use them', () => {
    const headers = ['問題', '選択肢', 'ID', '答え'];
    expect(suggestMapping(headers, previous).choiceSeparator).toBe('|');
    expect(
      suggestMapping(headers, { ...previous, choices: [], choiceSeparator: null }).choices,
    ).toEqual([]);
    expect(suggestMapping(['問題', '答え'], previous).choiceSeparator).toBeNull();
  });

  it('rejects sources too short to assign question and answer', () => {
    expect(() => suggestMapping([], null)).toThrow();
    expect(() => suggestMapping(['ID'], null)).toThrow();
  });
});
