use crate::agent::types::{AgentEvent, ClaudeEvent, ContentBlock};
use dioxus::prelude::*;
use std::collections::HashMap;

pub const BASE_CSS: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { height: 100%; overflow: hidden; }

body {
    background: var(--bg-primary);
    color: var(--fg-primary);
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    font-size: 13px;
}

/* ── IDE shell ── */
.ide {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
}

/* ── Title bar ── */
.titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    padding: 0 12px;
    background: var(--bg-tertiary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    -webkit-app-region: drag;
}
.titlebar-left { display: flex; align-items: center; gap: 8px; }
.titlebar-icon {
    color: var(--blue);
    font-family: monospace;
    font-weight: 700;
    font-size: 14px;
}
.titlebar-name {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-secondary);
    letter-spacing: 0.3px;
}
.titlebar-right { -webkit-app-region: no-drag; }
.titlebar-select {
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--fg-muted);
    font-size: 11px;
    cursor: pointer;
    outline: none;
}

/* ── IDE body ── */
.ide-body {
    display: flex;
    flex: 1;
    min-height: 0;
}

/* ── Sidebar ── */
.sidebar {
    width: 220px;
    height: 100%;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
}

.sidebar-section {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
}
.sidebar-section-grow {
    flex: 3;
    min-height: 150px;
    border-bottom: none;
    overflow-y: auto;
}

.section-header {
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-muted);
    letter-spacing: 0.8px;
    text-transform: uppercase;
    margin-bottom: 8px;
}

.section-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
}
.section-header-row .section-header { margin-bottom: 0; }

.sidebar-controls { display: flex; flex-direction: column; gap: 6px; }

.control-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--fg-secondary);
}
.control-label { font-size: 12px; color: var(--fg-muted); }

.checkbox-row {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--fg-secondary);
    cursor: pointer;
}
.checkbox-row input { accent-color: var(--blue); }

.select {
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--fg-primary);
    font-size: 12px;
    outline: none;
}

.text-input {
    width: 100%;
    padding: 4px 6px;
    border-radius: 3px;
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--fg-primary);
    font-size: 12px;
    outline: none;
}
.text-input::placeholder { color: var(--fg-muted); }
.text-input:focus,
.select:focus { border-color: var(--blue); }

.advanced-controls {
    overflow: hidden;
    max-height: 0;
    opacity: 0;
    margin-top: 0;
    pointer-events: none;
    transition: max-height 0.16s ease, opacity 0.12s ease, margin-top 0.16s ease;
}
.advanced-controls-open {
    max-height: 320px;
    opacity: 1;
    margin-top: 4px;
    pointer-events: auto;
}
.advanced-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg-tertiary) 82%, transparent);
}
.advanced-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.advanced-hint {
    font-size: 10px;
    color: var(--fg-muted);
    line-height: 1.4;
}

.tracker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    margin-bottom: 4px;
}
.tracker-info {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    font-size: 12px;
    color: var(--fg-primary);
    font-family: monospace;
}
.tracker-num { font-weight: 600; flex-shrink: 0; }
.tracker-title {
    color: var(--fg-muted);
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.sidebar-buttons { display: flex; gap: 4px; }

.btn {
    border: 1px solid var(--border);
    border-radius: 3px;
    background: var(--bg-tertiary);
    color: var(--fg-secondary);
    cursor: pointer;
    font-size: 11px;
    transition: background 0.1s;
}
.btn:hover { background: var(--border); color: var(--fg-primary); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-sm { padding: 3px 10px; }
.btn-xs { padding: 1px 6px; font-size: 10px; flex-shrink: 0; }
.btn-go { border-color: var(--green); color: var(--green); }
.btn-go:hover { background: var(--green); color: var(--bg-primary); }
.btn-strategy { border-color: var(--cyan); color: var(--cyan); width: 100%; }
.btn-strategy:hover { background: var(--cyan); color: var(--bg-primary); }
.btn-action { border-color: var(--purple); color: var(--purple); width: 100%; }
.btn-action:hover { background: var(--purple); color: var(--bg-primary); }
.btn-retro { border-color: var(--yellow); color: var(--yellow); width: 100%; }
.btn-retro:hover { background: var(--yellow); color: var(--bg-primary); }
.btn-ideation { border-color: var(--magenta); color: var(--magenta); width: 100%; }
.btn-ideation:hover { background: var(--magenta); color: var(--bg-primary); }
.btn-report { border-color: var(--blue); color: var(--blue); width: 100%; }
.btn-report:hover { background: var(--blue); color: var(--bg-primary); }
.btn-security { border-color: var(--orange); color: var(--orange); width: 100%; }
.btn-security:hover { background: var(--orange); color: var(--bg-primary); }
.btn-interview { border-color: var(--cyan); color: var(--cyan); width: 100%; }
.btn-interview:hover { background: var(--cyan); color: var(--bg-primary); }
.btn-merge { border-color: var(--green); color: var(--green); width: 100%; }
.btn-merge:hover { background: var(--green); color: var(--bg-primary); }
.btn-merge-active { background: var(--green); color: var(--bg-primary); opacity: 0.9; }
.btn-stop { border-color: var(--red); color: var(--red); width: 100%; }
.btn-stop:hover { background: var(--red); color: var(--bg-primary); }
.sidebar-buttons-col { flex-direction: column; }
.sidebar-buttons-divider {
    border: none;
    border-top: 1px solid var(--border);
    margin: 6px 2px;
    width: auto;
}

/* ── Feedback ── */
.feedback-hint {
    font-size: 11px;
    color: var(--fg-muted);
    margin-bottom: 6px;
    line-height: 1.4;
}
.feedback-input {
    width: 100%;
    min-height: 80px;
    padding: 6px 8px;
    border-radius: 3px;
    border: 1px solid var(--cyan);
    background: var(--bg-tertiary);
    color: var(--fg-primary);
    font-family: inherit;
    font-size: 12px;
    resize: vertical;
    outline: none;
    margin-bottom: 6px;
}
.feedback-input::placeholder { color: var(--fg-muted); }
.feedback-input:focus { border-color: var(--blue); }

/* ── Issue comment reminder ── */
.issue-comment-details {
    position: relative;
    display: inline-block;
}
.issue-comment-summary {
    list-style: none;
    cursor: pointer;
    font-size: 12px;
    opacity: 0.6;
    transition: opacity 0.1s;
    user-select: none;
}
.issue-comment-summary::-webkit-details-marker { display: none; }
.issue-comment-summary:hover { opacity: 1; }

.issue-comment-details[open] .issue-comment-reminder {
    display: block;
}

.issue-comment-reminder {
    position: absolute;
    top: 20px;
    right: 0;
    width: 200px;
    z-index: 100;
    border: 1px solid color-mix(in srgb, var(--yellow) 28%, var(--border));
    border-radius: 4px;
    background: var(--bg-secondary);
    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    padding: 8px 9px;
    display: none;
}
.issue-comment-reminder-title {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: var(--yellow);
    margin-bottom: 4px;
}
.issue-comment-reminder-copy,
.issue-comment-reminder-footer {
    font-size: 11px;
    line-height: 1.4;
    color: var(--fg-secondary);
}
.issue-comment-reminder-list {
    margin: 6px 0 0 16px;
    padding: 0;
}
.issue-comment-reminder-list li {
    margin-bottom: 4px;
    font-size: 11px;
    line-height: 1.3;
    color: var(--fg-secondary);
}
.issue-comment-reminder-footer {
    margin-top: 6px;
    color: var(--fg-muted);
}

/* ── Bot setup collapsible ── */
.bot-setup-details { width: 100%; }
.bot-setup-details summary { list-style: none; cursor: pointer; }
.bot-setup-details summary::-webkit-details-marker { display: none; }
.bot-setup-details[open] > :not(summary) { margin-top: 8px; }

/* ── Issue tree ── */
.issue-tree { list-style: none; }
.issue-node {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 0;
    font-size: 12px;
    color: var(--fg-secondary);
    min-width: 0;
    flex-wrap: wrap;
}
.issue-start { margin-left: auto; }
.dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
}
.dot-ready   { background: var(--green); }
.dot-blocked { background: var(--red); }
.issue-num { font-family: monospace; font-weight: 600; color: var(--fg-primary); }
.issue-title {
    font-size: 11px;
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
}
.issue-pr {
    font-size: 10px;
    font-family: monospace;
    color: var(--cyan);
    background: color-mix(in srgb, var(--cyan) 12%, transparent);
    border-radius: 3px;
    padding: 0 4px;
    flex-shrink: 0;
}
.pr-thread-count {
    font-size: 10px;
    font-family: monospace;
    color: var(--orange, var(--red));
    flex-shrink: 0;
}
.issue-blockers {
    font-size: 10px;
    color: var(--red);
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.issue-blockers-label {
    color: var(--fg-muted);
    font-style: italic;
    font-family: inherit;
}
.issue-status { font-size: 10px; color: var(--fg-muted); font-style: italic; }
.pr-author {
    font-size: 10px;
    color: var(--fg-muted);
    font-family: monospace;
    font-style: italic;
    white-space: nowrap;
}

/* ── Editor area ── */
.editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
}

.tab-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    padding: 0 8px 0 0;
}
.tab {
    padding: 0 16px;
    height: 100%;
    display: flex;
    align-items: center;
    font-size: 12px;
    color: var(--fg-muted);
    border-right: 1px solid var(--border);
    cursor: default;
}
.tab-active {
    background: var(--bg-primary);
    color: var(--fg-primary);
    border-bottom: 2px solid var(--blue);
}

.tab-actions { display: flex; gap: 12px; }
.tab-check {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--fg-muted);
    cursor: pointer;
}
.tab-check input { accent-color: var(--blue); }

.editor-content {
    flex: 1;
    overflow-y: auto;
    padding: 10px 14px 40px 14px;
    font-family: "SF Mono", "Fira Code", "Cascadia Code", "Menlo", monospace;
    font-size: 12px;
    line-height: 1.5;
}

.editor-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-style: italic;
}

.text-muted { color: var(--fg-muted); }

/* ── Status bar ── */
.statusbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 24px;
    padding: 0 10px;
    background: var(--blue);
    color: var(--bg-primary);
    font-size: 11px;
    font-weight: 500;
    flex-shrink: 0;
}
.statusbar-left, .statusbar-right { display: flex; align-items: center; gap: 6px; }
.status-sep { opacity: 0.5; }
.status-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
}
.status-dot-idle   { background: var(--bg-primary); opacity: 0.6; }
.status-dot-active { background: var(--green); animation: pulse 1.2s ease-in-out infinite; }
@keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
}

/* ── Event rows ── */
.ev-log { color: var(--green); margin-bottom: 2px; }
.ev-log .tag { color: var(--fg-muted); margin-right: 6px; }

.ev-system {
    border-left: 2px solid var(--blue);
    margin-bottom: 6px;
    padding: 3px 8px;
    border-radius: 0 3px 3px 0;
    background: color-mix(in srgb, var(--blue) 8%, transparent);
}
.ev-system .label { color: var(--blue); font-weight: 700; font-size: 11px; }
.ev-system .meta  { font-size: 10px; color: var(--fg-muted); }

.ev-assistant {
    border-left: 2px solid var(--green);
    margin-bottom: 6px;
    padding: 3px 8px;
    border-radius: 0 3px 3px 0;
    background: color-mix(in srgb, var(--green) 5%, transparent);
}
.ev-assistant .label { color: var(--green); font-weight: 700; font-size: 11px; margin-bottom: 3px; }

.ev-user {
    border-left: 2px solid var(--yellow);
    margin-bottom: 6px;
    padding: 3px 8px;
    border-radius: 0 3px 3px 0;
    background: color-mix(in srgb, var(--yellow) 5%, transparent);
}
.ev-user .label { color: var(--yellow); font-weight: 700; font-size: 11px; margin-bottom: 3px; }

.ev-result {
    border-left: 2px solid var(--purple);
    margin-bottom: 6px;
    padding: 3px 8px;
    border-radius: 0 3px 3px 0;
    background: color-mix(in srgb, var(--purple) 8%, transparent);
}
.ev-result .label { color: var(--purple); font-weight: 700; font-size: 11px; }
.ev-result .summary { margin-top: 2px; font-size: 11px; font-style: italic; color: var(--fg-secondary); }

/* ── Content blocks ── */
.block-text { white-space: pre-wrap; margin-bottom: 6px; }

.block-thinking {
    margin-bottom: 6px;
    background: var(--bg-tertiary);
    padding: 4px 8px;
    border-radius: 3px;
}
.block-thinking summary {
    color: var(--fg-muted);
    cursor: pointer;
    font-size: 11px;
}
.block-thinking summary:hover { color: var(--fg-secondary); }
.block-thinking .content {
    color: var(--fg-muted);
    margin-top: 3px;
    font-style: italic;
    font-size: 11px;
}

.block-tool-use {
    margin-bottom: 6px;
    background: color-mix(in srgb, var(--magenta) 6%, transparent);
    padding: 4px 8px;
    border-radius: 3px;
    border: 1px solid color-mix(in srgb, var(--magenta) 15%, transparent);
}
.block-tool-use summary {
    color: var(--magenta);
    cursor: pointer;
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
}
.block-tool-use summary:hover { color: var(--fg-secondary); }
.block-tool-use .tool-badge {
    display: inline-block;
    background: color-mix(in srgb, var(--magenta) 20%, transparent);
    border-radius: 2px;
    padding: 0 4px;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
}
.block-tool-use .tool-target {
    color: var(--fg-secondary);
    font-weight: 400;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.block-tool-use pre {
    font-size: 11px;
    color: var(--fg-secondary);
    overflow-x: auto;
    white-space: pre-wrap;
    margin-top: 3px;
}

.block-tool-result {
    margin-bottom: 6px;
    background: color-mix(in srgb, var(--cyan) 5%, transparent);
    padding: 4px 8px;
    border-radius: 3px;
    border: 1px solid color-mix(in srgb, var(--cyan) 12%, transparent);
}
.block-tool-result summary {
    color: var(--cyan);
    cursor: pointer;
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
}
.block-tool-result summary:hover { color: var(--fg-secondary); }
.block-tool-result .result-badge {
    display: inline-block;
    border-radius: 2px;
    padding: 0 4px;
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    flex-shrink: 0;
}
.block-tool-result .result-badge-ok {
    background: color-mix(in srgb, var(--green) 20%, transparent);
    color: var(--green);
}
.block-tool-result .result-badge-err {
    background: color-mix(in srgb, var(--red) 20%, transparent);
    color: var(--red);
}
.block-tool-result .result-meta {
    color: var(--fg-muted);
    font-weight: 400;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.block-tool-result pre {
    font-size: 11px;
    color: var(--fg-muted);
    margin-top: 3px;
    overflow-x: auto;
    white-space: pre-wrap;
}

/* ── Changed files ── */
.files-summary {
    display: flex;
    gap: 12px;
    padding: 6px 0;
    margin-bottom: 8px;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
}
.file-stat {
    font-weight: 600;
    font-family: monospace;
}
.file-stat-created { color: var(--green); }
.file-stat-modified { color: var(--yellow); }
.file-stat-read { color: var(--fg-muted); }

.file-list {
    list-style: none;
}
.file-entry {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 0;
    font-size: 12px;
    font-family: monospace;
}
.file-kind {
    width: 14px;
    text-align: center;
    font-weight: 700;
    flex-shrink: 0;
}
.file-kind-created { color: var(--green); }
.file-kind-modified { color: var(--yellow); }
.file-kind-deleted { color: var(--red); }
.file-kind-read { color: var(--fg-muted); }
.file-path {
    color: var(--fg-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.tab { cursor: pointer; user-select: none; }
.tab:hover { color: var(--fg-primary); }

/* ── Security panel ── */
.security-panel { font-family: inherit; }

.sec-summary {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    flex-wrap: wrap;
}
.sec-stat { font-weight: 600; font-family: monospace; }
.sec-export { margin-left: auto; }

.score-badge {
    font-size: 11px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 3px;
    letter-spacing: 0.3px;
}
.score-good { background: color-mix(in srgb, var(--green) 20%, transparent); color: var(--green); }
.score-mixed { background: color-mix(in srgb, var(--yellow) 20%, transparent); color: var(--yellow); }
.score-bad { background: color-mix(in srgb, var(--red) 20%, transparent); color: var(--red); }

.sec-category { margin-bottom: 14px; }
.sec-category-header {
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-muted);
    letter-spacing: 0.8px;
    text-transform: uppercase;
    margin-bottom: 6px;
    padding-bottom: 3px;
    border-bottom: 1px solid var(--border);
}

.sec-finding {
    margin-bottom: 6px;
    padding: 6px 8px;
    border-radius: 3px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
}
.sec-finding-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 4px;
}
.sec-finding-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg-primary);
}
.sec-finding-desc {
    font-size: 11px;
    color: var(--fg-secondary);
    line-height: 1.5;
}

.sec-sev-badge, .sec-status-badge {
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 2px;
    letter-spacing: 0.5px;
    flex-shrink: 0;
}
.sev-critical { background: color-mix(in srgb, var(--red) 20%, transparent); color: var(--red); }
.sev-high { background: color-mix(in srgb, var(--orange) 20%, transparent); color: var(--orange); }
.sev-medium { background: color-mix(in srgb, var(--yellow) 20%, transparent); color: var(--yellow); }
.sev-low { background: color-mix(in srgb, var(--blue) 20%, transparent); color: var(--blue); }
.sev-info { background: color-mix(in srgb, var(--fg-muted) 15%, transparent); color: var(--fg-muted); }

.status-pass { background: color-mix(in srgb, var(--green) 20%, transparent); color: var(--green); }
.status-fail { background: color-mix(in srgb, var(--red) 20%, transparent); color: var(--red); }
.status-warn { background: color-mix(in srgb, var(--yellow) 20%, transparent); color: var(--yellow); }

.sec-remediation {
    margin-top: 4px;
    padding: 4px 6px;
    border-radius: 2px;
    background: color-mix(in srgb, var(--cyan) 6%, transparent);
    border-left: 2px solid var(--cyan);
    font-size: 11px;
    color: var(--fg-secondary);
    line-height: 1.4;
}
.sec-remediation-label {
    font-weight: 600;
    color: var(--cyan);
}

/* ── Scrollbar ── */
.editor-content::-webkit-scrollbar, .sidebar::-webkit-scrollbar { width: 8px; }
.editor-content::-webkit-scrollbar-track, .sidebar::-webkit-scrollbar-track {
    background: transparent;
}
.editor-content::-webkit-scrollbar-thumb, .sidebar::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 4px;
}
.editor-content::-webkit-scrollbar-thumb:hover, .sidebar::-webkit-scrollbar-thumb:hover {
    background: var(--fg-muted);
}

/* ── Interview dialog ── */
.interview-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    padding: 16px 20px;
    gap: 12px;
}
.interview-empty {
    color: var(--fg-muted);
    text-align: center;
    padding: 32px 16px;
    font-style: italic;
}
.interview-section-header {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: var(--cyan);
    padding: 8px 0 4px;
    border-bottom: 1px solid var(--border);
    margin-top: 8px;
}
.interview-turn {
    display: flex;
    flex-direction: column;
    max-width: 85%;
}
.interview-turn-agent {
    align-self: flex-start;
}
.interview-turn-user {
    align-self: flex-end;
}
.interview-bubble {
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-wrap: break-word;
}
.interview-bubble-agent {
    background: color-mix(in srgb, var(--cyan) 8%, var(--bg-secondary));
    border-left: 3px solid var(--cyan);
    color: var(--fg-primary);
}
.interview-bubble-user {
    background: color-mix(in srgb, var(--purple) 10%, var(--bg-secondary));
    border-right: 3px solid var(--purple);
    color: var(--fg-primary);
    text-align: left;
}
.interview-role {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.3px;
    text-transform: uppercase;
    margin-bottom: 4px;
    color: var(--fg-muted);
}
.interview-role-agent { color: var(--cyan); }
.interview-role-user { color: var(--purple); text-align: right; }
.interview-summary-card {
    margin-top: 16px;
    padding: 16px;
    border-radius: 6px;
    border: 1px solid var(--cyan);
    background: color-mix(in srgb, var(--cyan) 5%, var(--bg-secondary));
}
.interview-summary-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--cyan);
    margin-bottom: 8px;
    letter-spacing: 0.3px;
}
.interview-summary-body {
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-wrap: break-word;
    color: var(--fg-primary);
}
.interview-status {
    text-align: center;
    padding: 8px;
    font-size: 11px;
    color: var(--fg-muted);
    font-style: italic;
}
"#;

#[component]
pub fn EventRow(
    event: AgentEvent,
    expand_all: bool,
    tool_names: HashMap<String, String>,
) -> Element {
    match event {
        AgentEvent::Log(msg) => rsx! {
            div { class: "ev-log",
                span { class: "tag", "[LOG]" }
                "{msg}"
            }
        },
        AgentEvent::Claude(ev) => rsx! {
            ClaudeEventRow { ev, expand_all, tool_names }
        },
        AgentEvent::Done | AgentEvent::AwaitingFeedback(_) | AgentEvent::TrackerUpdate(_) => {
            rsx! {}
        }
    }
}

#[component]
pub fn ClaudeEventRow(
    ev: ClaudeEvent,
    expand_all: bool,
    tool_names: HashMap<String, String>,
) -> Element {
    match ev {
        ClaudeEvent::System {
            subtype,
            model,
            description,
            ..
        } => rsx! {
            div { class: "ev-system",
                div { class: "label", "SYSTEM: {subtype}" }
                if let Some(m) = model { div { class: "meta", "Model: {m}" } }
                if let Some(d) = description { div { "{d}" } }
            }
        },
        ClaudeEvent::Assistant { message } => rsx! {
            div { class: "ev-assistant",
                div { class: "label", "ASSISTANT" }
                for block in message.content {
                    ContentBlockRow { block, expand_all, tool_names: tool_names.clone() }
                }
            }
        },
        ClaudeEvent::User { message } => rsx! {
            div { class: "ev-user",
                div { class: "label", "USER" }
                for block in message.content {
                    ContentBlockRow { block, expand_all, tool_names: tool_names.clone() }
                }
            }
        },
        ClaudeEvent::Result {
            status, summary, ..
        } => rsx! {
            div { class: "ev-result",
                div { class: "label", "RESULT: {status}" }
                if let Some(s) = summary { div { class: "summary", "{s}" } }
            }
        },
    }
}

/// Generate a one-line summary for a tool use based on name + input params.
fn tool_use_summary(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Read" => {
            let path = input
                .get("file_path")
                .or(input.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let basename = path.rsplit('/').next().unwrap_or(path);
            if let Some(limit) = input.get("limit").and_then(|v| v.as_u64()) {
                format!("{basename} ({limit} lines)")
            } else {
                basename.to_string()
            }
        }
        "Write" => {
            let path = input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            path.rsplit('/').next().unwrap_or(path).to_string()
        }
        "Edit" => {
            let path = input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            path.rsplit('/').next().unwrap_or(path).to_string()
        }
        "Bash" => {
            let cmd = input.get("command").and_then(|v| v.as_str()).unwrap_or("?");
            let truncated: String = cmd.chars().take(60).collect();
            if cmd.len() > 60 {
                format!("{truncated}...")
            } else {
                truncated
            }
        }
        "Grep" | "Search" => {
            let pattern = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            let truncated: String = pattern.chars().take(40).collect();
            if pattern.len() > 40 {
                format!("/{truncated}.../")
            } else {
                format!("/{truncated}/")
            }
        }
        "Glob" => {
            let pattern = input.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            pattern.to_string()
        }
        "Agent" => {
            let desc = input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("subagent");
            desc.to_string()
        }
        _ => {
            // Generic: show first string value found
            if let Some(obj) = input.as_object() {
                for val in obj.values() {
                    if let Some(s) = val.as_str() {
                        let truncated: String = s.chars().take(50).collect();
                        return if s.len() > 50 {
                            format!("{truncated}...")
                        } else {
                            truncated
                        };
                    }
                }
            }
            String::new()
        }
    }
}

/// Generate a one-line summary for a tool result based on content + tool name.
fn tool_result_summary(tool_name: &str, content: &str) -> (String, bool) {
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len();
    let is_error = content.contains("ERROR")
        || content.contains("Error:")
        || content.contains("FAILED")
        || content.contains("error[E")
        || content.contains("Permission denied")
        || content.contains("No such file");

    let summary = match tool_name {
        "Read" => format!("{line_count} lines"),
        "Edit" => {
            if is_error {
                "failed".to_string()
            } else {
                "applied".to_string()
            }
        }
        "Bash" => {
            let first = lines.first().map(|l| l.trim()).unwrap_or("");
            let truncated: String = first.chars().take(50).collect();
            if line_count > 1 {
                format!("{truncated}... ({line_count} lines)")
            } else {
                truncated
            }
        }
        "Grep" | "Glob" | "Search" => {
            format!("{line_count} lines")
        }
        "Write" => {
            if is_error {
                "failed".to_string()
            } else {
                "written".to_string()
            }
        }
        _ => {
            if line_count > 1 {
                format!("{line_count} lines")
            } else {
                let truncated: String = content.chars().take(50).collect();
                if content.len() > 50 {
                    format!("{truncated}...")
                } else {
                    truncated
                }
            }
        }
    };

    (summary, is_error)
}

#[component]
pub fn ContentBlockRow(
    block: ContentBlock,
    expand_all: bool,
    tool_names: HashMap<String, String>,
) -> Element {
    match block {
        ContentBlock::Text { text } => rsx! {
            div { class: "block-text", "{text}" }
        },
        ContentBlock::Thinking { thinking } => rsx! {
            details { open: expand_all, class: "block-thinking",
                summary { "Thinking..." }
                div { class: "content", "{thinking}" }
            }
        },
        ContentBlock::ToolUse { id: _, name, input } => {
            let summary = tool_use_summary(&name, &input);
            rsx! {
                details { open: expand_all, class: "block-tool-use",
                    summary {
                        span { class: "tool-badge", "{name}" }
                        if !summary.is_empty() {
                            span { class: "tool-target", "{summary}" }
                        }
                    }
                    pre { "{serde_json::to_string_pretty(&input).unwrap_or_default()}" }
                }
            }
        }
        ContentBlock::ToolResult { id, content } => {
            let tool_name = tool_names.get(&id).map(|s| s.as_str()).unwrap_or("");
            let (summary, is_error) = tool_result_summary(tool_name, &content);
            let badge_class = if is_error {
                "result-badge result-badge-err"
            } else {
                "result-badge result-badge-ok"
            };
            let badge_text = if !tool_name.is_empty() {
                tool_name.to_string()
            } else {
                "Result".to_string()
            };
            rsx! {
                details { open: expand_all, class: "block-tool-result",
                    summary {
                        span { class: "{badge_class}", "{badge_text}" }
                        span { class: "result-meta", "{summary}" }
                    }
                    pre { "{content}" }
                }
            }
        }
    }
}
