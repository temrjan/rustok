/**
 * pickQuizQuestions — pure-function coverage.
 *
 * No React, no mocks — just verifies invariants over 200 random runs
 * (sample size chosen to surface rare distractor-collision bugs without
 * making the suite slow).
 */

import { BIP39_ENGLISH } from '../bip39Wordlist';
import { pickQuizQuestions } from '../pickQuizQuestions';

const TEST_MNEMONIC =
  'abandon ability able about above absent absorb abstract absurd abuse access accident';

const TEST_WORDS = TEST_MNEMONIC.split(' ');

const WORDLIST_SET = new Set(BIP39_ENGLISH);

describe('pickQuizQuestions', () => {
  it('returns exactly 3 questions', () => {
    const questions = pickQuizQuestions(TEST_MNEMONIC);
    expect(questions).toHaveLength(3);
  });

  it('throws when mnemonic is not 12 words', () => {
    expect(() => pickQuizQuestions('only three words here')).toThrow(
      /expected 12 words/,
    );
    expect(() =>
      pickQuizQuestions(`${TEST_MNEMONIC} extra`),
    ).toThrow(/expected 12 words/);
  });

  describe('invariants (200 random runs)', () => {
    const RUN_COUNT = 200;

    it('wordIndex values are unique and в range [0, 11]', () => {
      for (let run = 0; run < RUN_COUNT; run += 1) {
        const questions = pickQuizQuestions(TEST_MNEMONIC);
        const indices = questions.map((q) => q.wordIndex);
        expect(new Set(indices).size).toBe(3);
        for (const idx of indices) {
          expect(idx).toBeGreaterThanOrEqual(0);
          expect(idx).toBeLessThanOrEqual(11);
        }
      }
    });

    it('correctWord equals mnemonic word at wordIndex', () => {
      for (let run = 0; run < RUN_COUNT; run += 1) {
        const questions = pickQuizQuestions(TEST_MNEMONIC);
        for (const q of questions) {
          expect(q.correctWord).toBe(TEST_WORDS[q.wordIndex]);
        }
      }
    });

    it('each question has exactly 4 unique options including correctWord', () => {
      for (let run = 0; run < RUN_COUNT; run += 1) {
        const questions = pickQuizQuestions(TEST_MNEMONIC);
        for (const q of questions) {
          expect(q.options).toHaveLength(4);
          expect(new Set(q.options).size).toBe(4);
          expect(q.options).toContain(q.correctWord);
        }
      }
    });

    it('every distractor is in BIP39_ENGLISH и differs from correctWord', () => {
      for (let run = 0; run < RUN_COUNT; run += 1) {
        const questions = pickQuizQuestions(TEST_MNEMONIC);
        for (const q of questions) {
          const distractors = q.options.filter((o) => o !== q.correctWord);
          expect(distractors).toHaveLength(3);
          for (const d of distractors) {
            expect(WORDLIST_SET.has(d)).toBe(true);
            expect(d).not.toBe(q.correctWord);
          }
        }
      }
    });

    it('questions are returned sorted by wordIndex ascending', () => {
      for (let run = 0; run < RUN_COUNT; run += 1) {
        const indices = pickQuizQuestions(TEST_MNEMONIC).map(
          (q) => q.wordIndex,
        );
        const sorted = [...indices].sort((a, b) => a - b);
        expect(indices).toEqual(sorted);
      }
    });
  });
});
