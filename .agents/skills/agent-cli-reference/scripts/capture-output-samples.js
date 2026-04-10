#!/usr/bin/env node

const { execFileSync } = require("child_process");
const { writeFileSync, mkdirSync } = require("fs");
const { join } = require("path");

const dir = join(__dirname, "..", "references", "samples");
mkdirSync(dir, { recursive: true });

const P = "Respond with only the word: hello";
const agents = [
  ["claude", [
    ["text",        ["-p", "--dangerously-skip-permissions", "--output-format", "text", P]],
    ["json",        ["-p", "--dangerously-skip-permissions", "--output-format", "json", P]],
    ["stream-json", ["-p", "--dangerously-skip-permissions", "--verbose", "--output-format", "stream-json", P]],
  ]],
  ["codex", [
    ["text",  ["exec", "--full-auto", P]],
    ["jsonl", ["exec", "--full-auto", "--json", P]],
  ]],
  ["cline", [
    ["rich",  ["--yolo", "-F", "rich", P]],
    ["json",  ["--yolo", "-F", "json", P]],
    ["plain", ["--yolo", "-F", "plain", P]],
  ]],
  ["gemini", [
    ["text",        ["--yolo", "-o", "text", "-p", P]],
    ["json",        ["--yolo", "-o", "json", "-p", P]],
    ["stream-json", ["--yolo", "-o", "stream-json", "-p", P]],
  ]],
  ["junie", [
    ["text",        ["--brave", "--output-format", "text", "--task", P]],
    ["json",        ["--brave", "--output-format", "json", "--task", P]],
    ["json-stream", ["--brave", "--output-format", "json-stream", "--task", P]],
  ]],
  ["grok", [
    ["text", ["-p", P]],
  ]],
];

for (const [bin, formats] of agents) {
  for (const [fmt, args] of formats) {
    const file = join(dir, `${bin}-${fmt}.txt`);
    console.log(`${bin} ${fmt}...`);
    try {
      const out = execFileSync(bin, args, { encoding: "utf8", timeout: 60000 });
      writeFileSync(file, out);
    } catch (e) {
      writeFileSync(file, `ERROR: ${e.message}\n\n${e.stdout || ""}${e.stderr || ""}`);
    }
  }
}
console.log("Done.");