// Adapted from Sahand Sojoodi's X to PDF 1.0.1 (MIT).
// See licenses/XtoPDF-MIT.txt and NOTICE.md.
(() => {
  "use strict";
  const selectors = [
    '[data-testid="twitterArticleReadView"]',
    '[data-testid="twitterArticleRichText"]',
    '[data-testid="articleNoteTweet"]',
    'article [data-testid="longformRichTextComponent"]'
  ];
  const root = selectors.map(s => document.querySelector(s)).find(Boolean);
  if (!root) return {error: "No X Article found. Finish signing in, open the full article and try Capture again. Ordinary posts and threads are unsupported."};
  const clean = s => String(s || "").replace(/\u00a0/g, " ").trim();
  const inline = (node, depth = 0) => {
    if (depth > 24) throw new Error("Article formatting is too deeply nested.");
    if (node.nodeType === 3) return [{kind: "text", text: node.textContent || ""}];
    if (node.nodeType !== 1) return [];
    if (["SCRIPT", "STYLE", "IFRAME", "OBJECT", "SVG", "BUTTON", "INPUT"].includes(node.tagName)) return [];
    if (node.tagName === "BR") return [{kind: "br"}];
    const children = [...node.childNodes].flatMap(n => inline(n, depth + 1));
    const kind = {B: "strong", STRONG: "strong", I: "em", EM: "em", S: "s", CODE: "code", SUP: "sup", SUB: "sub"}[node.tagName];
    if (kind) return [{kind, children}];
    if (node.tagName === "A") return [{kind: "link", href: node.href || "", children}];
    return children;
  };
  try {
    const title = clean(root.querySelector("h1")?.textContent || document.title.replace(/\s+(on|\/)\s*X.*$/i, ""));
    const user = root.closest("article")?.querySelector('[data-testid="User-Name"]') || document.querySelector('[data-testid="User-Name"]');
    const byline = clean(user?.innerText).split("\n").filter(Boolean).slice(0, 2).join(" ");
    // One DOM-ordered traversal. A code block is emitted in place, never appended.
    const candidates = [...root.querySelectorAll('[data-block="true"], [data-testid="markdown-code-block"]')];
    const content = [];
    for (const node of candidates) {
      const codeRoot = node.closest('[data-testid="markdown-code-block"]');
      if (codeRoot && codeRoot !== node) continue;
      if (codeRoot === node) {
        const text = (node.querySelector("pre")?.textContent || "").trimEnd();
        if (text) content.push({kind: "code", text});
        continue;
      }
      // Render leaf blocks only; nested wrappers otherwise duplicate the text.
      if (node.querySelector('[data-block="true"], [data-testid="markdown-code-block"]')) continue;
      const text = clean(node.textContent);
      if (!text) continue;
      const semantic = node.closest("h1,h2,h3,h4,h5,h6,li,blockquote") || node;
      const tag = semantic.tagName.toLowerCase();
      if (tag === "h1" && text === title) continue;
      const kind = /^h[1-6]$/.test(tag) ? "heading" : tag === "li" ? "list-item" : tag === "blockquote" ? "quote" : "paragraph";
      content.push({kind, level: kind === "heading" ? Number(tag[1]) : 0,
        ordered: kind === "list-item" && semantic.parentElement?.tagName === "OL",
        runs: [...node.childNodes].flatMap(n => inline(n))});
    }
    if (!content.length) return {error: "The article text is not loaded. Scroll through it, then try Capture again."};
    const article = {title: title || "X Article", byline, source: location.href, content};
    if (JSON.stringify(article).length > 2000000) return {error: "This article exceeds the 2 MB capture limit."};
    return {article};
  } catch (error) { return {error: String(error.message).slice(0, 300)}; }
})()
