/**
 * Demo/seed data. Runs once on first launch (guarded by a localStorage flag) and
 * only when the library is empty, so a new user immediately sees how Recall works.
 * Deleting everything won't re-seed, because the flag persists.
 */

import { api } from "./api";
import type { CardInput } from "./db/models";

interface SeedItem {
  title: string;
  content: string;
  tags: string[];
  cards: Omit<CardInput, "knowledge_item_id">[];
}

const SEED: SeedItem[] = [
  {
    title: "TCP Three-Way Handshake",
    tags: ["networking", "cs"],
    content:
      "TCP establishes a reliable connection using a three-way handshake. The client sends a SYN " +
      "segment to open the connection and propose an initial sequence number. The server replies " +
      "with SYN-ACK, acknowledging the client's SYN and sending its own. The client responds with " +
      "ACK, confirming the server's sequence number. After these three steps, both sides have " +
      "synchronized sequence numbers and the connection is established.",
    cards: [
      {
        question: "What are the three steps of the TCP handshake?",
        answer: "SYN → SYN-ACK → ACK",
        type: "basic",
        difficulty: "easy",
      },
      {
        question: "TCP establishes a connection using {{SYN}}, {{SYN-ACK}}, and {{ACK}}.",
        answer: "SYN, SYN-ACK, ACK",
        type: "cloze",
        difficulty: "easy",
      },
      {
        question: "Which flag does the client send first to initiate a TCP connection?",
        answer: "SYN",
        type: "mcq",
        difficulty: "medium",
        metadata: { distractors: ["ACK", "FIN", "RST"] },
      },
      {
        question: "Explain why TCP uses a three-way handshake.",
        answer:
          "To synchronize initial sequence numbers and confirm that both sides can send and receive, " +
          "establishing a reliable, ordered connection before data transfer.",
        type: "free",
        difficulty: "hard",
      },
      {
        question: "Three-way handshake",
        answer: "The process TCP uses to establish a connection: SYN, then SYN-ACK, then ACK.",
        type: "reverse",
        difficulty: "medium",
      },
    ],
  },
  {
    title: "Photosynthesis",
    tags: ["biology"],
    content:
      "Photosynthesis is the process by which green plants convert light energy into chemical energy. " +
      "In the chloroplasts, the pigment chlorophyll absorbs light, which drives the conversion of " +
      "carbon dioxide and water into glucose and oxygen. The glucose stores energy; the oxygen is " +
      "released as a by-product.",
    cards: [
      {
        question: "What is the overall word equation for photosynthesis?",
        answer: "Carbon dioxide + water → glucose + oxygen (using light energy).",
        type: "basic",
        difficulty: "medium",
      },
      {
        question: "Photosynthesis occurs in the {{chloroplast}} and uses the pigment {{chlorophyll}}.",
        answer: "chloroplast, chlorophyll",
        type: "cloze",
        difficulty: "easy",
      },
      {
        question: "Which gas is released as a by-product of photosynthesis?",
        answer: "Oxygen",
        type: "mcq",
        difficulty: "easy",
        metadata: { distractors: ["Carbon dioxide", "Nitrogen", "Hydrogen"] },
      },
    ],
  },
];

const SEED_FLAG = "recall.seeded.v1";

export async function maybeSeed(): Promise<boolean> {
  // Only seed once per install.
  try {
    if (localStorage.getItem(SEED_FLAG)) return false;
  } catch {
    /* localStorage may be unavailable; fall through and check emptiness */
  }

  let empty = false;
  try {
    const items = await api.listItems();
    empty = items.length === 0;
  } catch {
    return false; // if we can't check, don't seed
  }
  if (!empty) {
    try {
      localStorage.setItem(SEED_FLAG, "1");
    } catch {
      /* ignore */
    }
    return false;
  }

  try {
    for (const seed of SEED) {
      const item = await api.createItem({ title: seed.title, content: seed.content, tags: seed.tags });
      for (const c of seed.cards) {
        await api.createCard({ ...c, knowledge_item_id: item.id });
      }
    }
    try {
      localStorage.setItem(SEED_FLAG, "1");
    } catch {
      /* ignore */
    }
    return true;
  } catch {
    return false;
  }
}
