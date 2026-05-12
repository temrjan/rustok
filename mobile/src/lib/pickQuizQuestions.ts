/**
 * pickQuizQuestions — Phase 4 M3.3.
 *
 * Pure helper for `QuizScreen`: given the 12-word BIP-39 mnemonic
 * revealed in M3.2, produce 3 multiple-choice questions covering
 * randomly-selected word positions, each with the correct word plus
 * 3 unique distractors sampled from `BIP39_ENGLISH`.
 *
 * No React, no hooks, no global state — fully unit-testable.
 *
 * Randomness uses `Math.random` rather than crypto-rng intentionally.
 * Quiz failure mode is "user guesses wrong" → re-prompt; non-uniform
 * sampling does not weaken any security property of the wallet.
 *
 * Questions are returned sorted by `wordIndex` ascending — questions
 * appear in mnemonic order, which reads more naturally than а random
 * ordering of "Word 7, Word 2, Word 11".
 *
 * @see docs/PHASE4-DESIGN-ONBOARDING.md § 4.6 (QuizScreen spec)
 */

import { BIP39_ENGLISH } from './bip39Wordlist';

const MNEMONIC_LENGTH = 12;
const QUESTION_COUNT = 3;
const OPTIONS_PER_QUESTION = 4;
const DISTRACTORS_PER_QUESTION = OPTIONS_PER_QUESTION - 1;

export interface QuizQuestion {
  readonly wordIndex: number;
  readonly correctWord: string;
  readonly options: readonly string[];
}

export function pickQuizQuestions(mnemonic: string): QuizQuestion[] {
  const words = mnemonic.split(' ');
  if (words.length !== MNEMONIC_LENGTH) {
    throw new Error(
      `pickQuizQuestions: expected ${MNEMONIC_LENGTH} words, got ${words.length}`,
    );
  }
  const indices = pickUniqueIndices();
  return indices.map((wordIndex) => {
    const correctWord = words[wordIndex];
    if (correctWord === undefined) {
      throw new Error(
        `pickQuizQuestions: word at index ${wordIndex} is undefined`,
      );
    }
    const distractors = pickDistractors(correctWord);
    const options = shuffleArray([correctWord, ...distractors]);
    return { wordIndex, correctWord, options };
  });
}

function pickUniqueIndices(): number[] {
  const set = new Set<number>();
  while (set.size < QUESTION_COUNT) {
    set.add(Math.floor(Math.random() * MNEMONIC_LENGTH));
  }
  return Array.from(set).sort((a, b) => a - b);
}

function pickDistractors(correctWord: string): string[] {
  const set = new Set<string>();
  while (set.size < DISTRACTORS_PER_QUESTION) {
    const candidate =
      BIP39_ENGLISH[Math.floor(Math.random() * BIP39_ENGLISH.length)];
    if (candidate === undefined || candidate === correctWord) continue;
    set.add(candidate);
  }
  return Array.from(set);
}

function shuffleArray<T>(arr: readonly T[]): T[] {
  const result = [...arr];
  for (let i = result.length - 1; i > 0; i -= 1) {
    const j = Math.floor(Math.random() * (i + 1));
    const tmp = result[i] as T;
    result[i] = result[j] as T;
    result[j] = tmp;
  }
  return result;
}
