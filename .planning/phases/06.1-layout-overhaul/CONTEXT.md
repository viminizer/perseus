# Phase 6.1: Layout Overhaul — Context

## Vision

Postman-style layout with a persistent sidebar and redesigned request input area. This phase establishes the structural foundation before Collections fills it with content.

## How It Works

### Sidebar
- Empty placeholder panel on the left
- Structure ready for Collections phase to populate
- Persistent across the application

### Request Input Row
- Horizontal layout: `[Method] [URL Input] [Send]`
- Method selector: compact width (fits longest method name)
- URL input: fills remaining space
- Send button: small, end of row

### Method Selection Popup
- Focus method box → press Enter → popup appears
- Popup shows list of HTTP methods with distinct colors per method
- Navigate with j/k (vim-style)
- Press Enter to select → popup closes, method displayed

## What's Essential

- Sidebar structure (empty, ready for content)
- Horizontal request input layout
- Method selector popup with colored methods and j/k navigation
- Clean visual hierarchy

## Out of Scope

- Any persistence/saving integration
- Populating the sidebar with actual collections
- Tabs for multiple requests
- Response pane changes (unless needed for layout balance)

---
*Created: 2026-02-06*
