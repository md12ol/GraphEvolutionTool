/* GET documentation site — shared behaviour.
   Everything is inlined here so the site works from `file://` as well as from a
   local HTTP server: no fetch, no external libraries, no build step.

   Each page declares where it sits with `<body data-page="guide/pipeline.html">`.
   That single attribute drives the sidebar's link prefixes, the active-page
   highlight, and the prev/next pager. Add a page to NAV below and it appears
   everywhere. */

(function () {
  "use strict";

  /* ---------- the site map -------------------------------------------- */

  /* A group with no title renders as bare links with no collapsing header --
     Overview and Design Notes are each one page, not a section, and a folder
     around a single page was noise. */
  var NAV = [
    {
      title: null,
      items: [
        ["index.html",                        "Overview", "what is get graph evolution tool"]
      ]
    },
    {
      title: "Start Here",
      items: [
        ["guide/getting-started.html",        "Getting Started", "install quickstart first run"],
        ["guide/choosing-a-route.html",       "Choose a Route", "python toml rust cli library"],
        ["guide/concepts.html",               "Concepts & Vocabulary", "graph genome fitness objective evolution glossary"],
        ["guide/troubleshooting.html",        "Troubleshooting", "error install import config slow output"]
      ]
    },
    {
      title: "Learn",
      items: [
        ["guide/pipeline.html",               "The Pipeline", "loop graph genome expression"],
        ["guide/variation.html",              "Variation & Selection", "crossover mutation breeding replacement"],
        ["guide/fitness.html",                "Fitness & Output", "objective sir seed result log reproducibility"]
      ]
    },
    {
      title: "Use Python",
      items: [
        ["guide/route-python-objects.html",   "Config Objects", "python typed objects graphevolver"],
        ["guide/route-python-toml.html",      "TOML File", "python configuration file"],
        ["guide/example-bundle.html",         "Example Bundle", "download examples plots analysis configs"],
        ["guide/config-builder.html",         "Config Builder", "build generate config toml form interactive"]
      ]
    },
    {
      title: "Use Rust",
      items: [
        ["guide/route-rust-library.html",     "As a Library", "crate native objective embedded", "source only"],
        ["guide/route-rust-cli.html",         "get-run CLI", "command line config output", "source only"]
      ]
    },
    {
      title: "Reference",
      items: [
        ["guide/data-and-inputs.html",        "Data & Inputs", "edge file graph biological gene protein identifiers mapping limits"],
        ["guide/configuration.html",          "Configuration Reference", "keys fields defaults validation toml rust cli"]
      ]
    },
    {
      title: "Contribute",
      items: [
        ["guide/contributing.html",           "Contributor Guide", "clone test docs pull request source map"],
        ["guide/extending.html",              "Extension Map", "add feature cost dispatch"],
        ["guide/new-fitness.html",            "Add an Objective", "python callable rust fitness"]
      ]
    },
    {
      title: "Advanced Extensions",
      items: [
        ["guide/new-genome.html",             "Add a Genome", "representation graph"],
        ["guide/new-evolver.html",            "Add a Strategy", "evolution algorithm"],
        ["guide/new-selection.html",          "Add a Selection Scheme", "parents"],
        ["guide/new-scope.html",              "Add a Scope", "population slice"],
        ["guide/new-replacement.html",        "Add a Replacement Policy", "overwrite children"],
        ["guide/new-crossover.html",          "Add a Crossover Operator", "recombination"],
        ["guide/new-mutation.html",           "Add a Mutation Operator", "variation"]
      ]
    },
    {
      title: null,
      items: [
        ["design-notes.html",                 "Design Notes"]
      ]
    }
  ];

  /* ---------- where am I ----------------------------------------------- */

  var page = document.body.getAttribute("data-page") || "index.html";
  var depth = page.split("/").length - 1;
  var root = depth === 0 ? "" : new Array(depth + 1).join("../");

  function href(target) { return root + target; }

  function setTheme(theme) {
    document.documentElement.setAttribute("data-theme", theme);
    try { localStorage.setItem("get-docs-theme", theme); } catch (e) { /* file:// may deny storage */ }
  }

  function initialTheme() {
    try { return localStorage.getItem("get-docs-theme") || "light"; }
    catch (e) { return "light"; }
  }

  /* ---------- sidebar --------------------------------------------------- */

  function buildSidebar() {
    var aside = document.createElement("aside");
    aside.className = "sidebar";
    aside.id = "sidebar";

    var brand = document.createElement("div");
    brand.className = "brand";
    brand.innerHTML =
      '<svg class="mark" viewBox="0 0 32 32" aria-hidden="true">' +
      '<g class="d-stroke-acc" stroke-width="1.6">' +
      '<path d="M8 22 L16 8 L24 22 Z"/><path d="M8 22 L24 22"/>' +
      '</g>' +
      '<g fill="currentColor" style="color:var(--accent)">' +
      '<circle cx="16" cy="8" r="3"/><circle cx="8" cy="22" r="3"/><circle cx="24" cy="22" r="3"/>' +
      '</g></svg>' +
      '<a href="' + href("index.html") + '">GET Docs</a>' +
      '<span class="tag">v0.9.0</span>';
    aside.appendChild(brand);

    /* The repository, once, in the site chrome -- so every page can reach the
       source without any route page having to assume the reader has a clone.
       Same tab deliberately: nothing here is a form to lose. */
    var repo = document.createElement("a");
    repo.className = "brand-repo";
    repo.href = "https://github.com/md12ol/GraphEvolutionTool";
    repo.textContent = "Source on GitHub \u2197";
    aside.appendChild(repo);

    var tools = document.createElement("div");
    tools.className = "sidebar-tools";
    var label = document.createElement("label");
    label.className = "sr-only";
    label.setAttribute("for", "page-search");
    label.textContent = "Find a documentation page";
    var search = document.createElement("input");
    search.id = "page-search";
    search.type = "search";
    search.placeholder = "Find a page…";
    search.setAttribute("autocomplete", "off");
    tools.appendChild(label);
    tools.appendChild(search);
    var searchStatus = document.createElement("span");
    searchStatus.className = "sr-only";
    searchStatus.setAttribute("aria-live", "polite");
    tools.appendChild(searchStatus);

    var theme = document.createElement("button");
    theme.className = "theme-toggle";
    theme.type = "button";
    function updateThemeLabel() {
      var dark = document.documentElement.getAttribute("data-theme") === "dark";
      theme.textContent = dark ? "Use light theme" : "Use dark theme";
      theme.setAttribute("aria-label", theme.textContent);
    }
    updateThemeLabel();
    theme.addEventListener("click", function () {
      setTheme(document.documentElement.getAttribute("data-theme") === "dark" ? "light" : "dark");
      updateThemeLabel();
    });
    tools.appendChild(theme);
    aside.appendChild(tools);

    /* One section open at a time: the one holding the current page. A title is
       a link to its own first page, so following it lands you there and the
       next page load opens that section and closes this one -- the accordion
       needs no state of its own, and a shared link always arrives showing the
       same thing. */
    var nav = document.createElement("nav");
    nav.setAttribute("aria-label", "Documentation");
    NAV.forEach(function (group) {
      var holdsPage = group.items.some(function (i) { return i[0] === page; });

      var g = document.createElement("div");
      // A titleless group has nothing to collapse, so it is always open.
      var startsOpen = holdsPage || !group.title || (page === "index.html" && group.title === "Start Here");
      g.className = "nav-group" + (startsOpen ? " open" : "");
      g.setAttribute("data-default-open", startsOpen ? "true" : "false");

      if (group.title) {
        var t = document.createElement("button");
        t.className = "nav-title";
        t.type = "button";
        t.textContent = group.title;
        t.setAttribute("aria-expanded", startsOpen ? "true" : "false");
        t.addEventListener("click", function () {
          var open = !g.classList.contains("open");
          g.classList.toggle("open", open);
          t.setAttribute("aria-expanded", open ? "true" : "false");
        });
        g.appendChild(t);
      }

      var list = document.createElement("div");
      list.className = "nav-items";
      group.items.forEach(function (item) {
        var a = document.createElement("a");
        a.className = "nav-link" + (item[0] === page ? " active" : "");
        a.href = href(item[0]);
        a.textContent = item[1];
        a.setAttribute("data-search", (item[1] + " " + (item[2] || "")).toLowerCase());
        if (item[0] === page) a.setAttribute("aria-current", "page");
        if (item[3]) {
          var status = document.createElement("span");
          status.className = "nav-status";
          status.textContent = item[3];
          a.appendChild(status);
        }
        list.appendChild(a);
      });
      g.appendChild(list);
      nav.appendChild(g);
    });
    aside.appendChild(nav);

    search.addEventListener("input", function () {
      var query = search.value.trim().toLowerCase();
      var matches = 0;
      Array.prototype.forEach.call(nav.querySelectorAll(".nav-group"), function (group) {
        var links = group.querySelectorAll(".nav-link");
        var any = false;
        Array.prototype.forEach.call(links, function (link) {
          var match = !query || link.getAttribute("data-search").indexOf(query) !== -1;
          link.hidden = !match;
          any = any || match;
          if (match && query) matches += 1;
        });
        group.hidden = !any;
        if (query && any) group.classList.add("open");
        if (!query) group.classList.toggle("open", group.getAttribute("data-default-open") === "true");
        var title = group.querySelector(".nav-title");
        if (title) title.setAttribute("aria-expanded", group.classList.contains("open") ? "true" : "false");
      });
      searchStatus.textContent = query ? (matches + (matches === 1 ? " page found" : " pages found")) : "Page filter cleared";
    });
    return aside;
  }

  /* ---------- on-page table of contents -------------------------------- */

  function slugify(s) {
    return s.toLowerCase().replace(/[^\w\s-]/g, "").trim().replace(/\s+/g, "-");
  }

  function buildToc(main) {
    /* Card headings are navigation, not sections — they would otherwise fill the
       table of contents with links that duplicate the sidebar. */
    var heads = [];
    Array.prototype.forEach.call(main.querySelectorAll("h2, h3"), function (h) {
      if (!h.closest(".card") && !h.closest(".path-card")) heads.push(h);
    });
    if (heads.length < 3) return null;

    var toc = document.createElement("nav");
    toc.className = "toc";
    toc.setAttribute("aria-label", "On this page");
    var title = document.createElement("button");
    title.className = "toc-toggle";
    title.type = "button";
    title.textContent = "On this page";
    toc.appendChild(title);
    var list = document.createElement("div");
    list.className = "toc-links";
    toc.appendChild(list);

    Array.prototype.forEach.call(heads, function (h) {
      if (!h.id) h.id = slugify(h.textContent);
      var a = document.createElement("a");
      a.href = "#" + h.id;
      a.textContent = h.textContent.replace(/\s*#\s*$/, "");
      a.className = h.tagName === "H3" ? "lvl-3" : "lvl-2";
      list.appendChild(a);

      var link = document.createElement("a");
      link.className = "anchor";
      link.href = "#" + h.id;
      link.textContent = "#";
      link.setAttribute("aria-label", "Permalink to " + h.textContent.replace(/\s*#\s*$/, ""));
      h.appendChild(link);
    });
    function setTocOpen(open) {
      list.hidden = !open;
      title.setAttribute("aria-expanded", open ? "true" : "false");
    }
    var wideToc = window.matchMedia("(min-width: 1101px)");
    setTocOpen(wideToc.matches);
    var syncToc = function (event) { setTocOpen(event.matches); };
    if (wideToc.addEventListener) wideToc.addEventListener("change", syncToc);
    else wideToc.addListener(syncToc);
    title.addEventListener("click", function () {
      setTocOpen(title.getAttribute("aria-expanded") !== "true");
    });
    return toc;
  }

  function trackToc(toc, main) {
    var links = toc.querySelectorAll("a");
    var targets = [];
    Array.prototype.forEach.call(links, function (a) {
      var el = main.querySelector(a.getAttribute("href"));
      if (el) targets.push([el, a]);
    });
    function update() {
      var best = null;
      targets.forEach(function (pair) {
        if (pair[0].getBoundingClientRect().top < 120) best = pair[1];
      });
      Array.prototype.forEach.call(links, function (a) { a.classList.remove("active"); });
      if (best) best.classList.add("active");
    }
    window.addEventListener("scroll", update, { passive: true });
    update();
  }

  /* ---------- prev / next ---------------------------------------------- */

  function flatNav() {
    var flat = [];
    NAV.forEach(function (g) {
      g.items.forEach(function (i) { flat.push(i); });
    });
    return flat;
  }

  function buildPager() {
    var flat = flatNav();
    var idx = -1;
    flat.forEach(function (i, n) { if (i[0] === page) idx = n; });
    if (idx === -1) return null;

    var pager = document.createElement("nav");
    pager.className = "pager";
    if (idx > 0) {
      var p = flat[idx - 1];
      var a = document.createElement("a");
      a.href = href(p[0]);
      a.innerHTML = '<span class="dir">Previous</span>' + p[1];
      pager.appendChild(a);
    }
    if (idx < flat.length - 1) {
      var n2 = flat[idx + 1];
      var b = document.createElement("a");
      b.className = "to-next";
      b.href = href(n2[0]);
      b.innerHTML = '<span class="dir">Next</span>' + n2[1];
      pager.appendChild(b);
    }
    return pager.children.length ? pager : null;
  }

  /* ---------- code blocks: language tag, copy button, light tinting ----- */

  var KEYWORDS = {
    rust: ["pub","fn","let","mut","struct","enum","impl","trait","for","in","if",
           "else","match","return","self","Self","use","mod","const","type","where",
           "dyn","Box","move","as","crate","ref","loop","while","true","false"],
    python: ["import","from","def","class","return","if","else","elif","for","in",
             "with","as","None","True","False","not","and","or","lambda","print"],
    toml: []
  };

  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  /* A deliberately small highlighter: comments, strings, numbers, keywords.
     It tints; it does not parse. Anything it gets wrong is cosmetic. */
  /* Apply `re` to the parts of `html` that are not inside a tag, so a
     replacement cannot corrupt an attribute inserted by an earlier pass. */
  function outsideTags(html, re, replacement) {
    var parts = html.split(/(<[^>]*>)/);
    for (var i = 0; i < parts.length; i++) {
      if (parts[i].charAt(0) === "<") continue;
      parts[i] = parts[i].replace(re, replacement);
    }
    return parts.join("");
  }

  function tint(src, lang) {
    var out = escapeHtml(src);
    var commentRe = lang === "python" || lang === "toml" || lang === "bash"
      ? /(^|\n)(\s*#[^\n]*)/g
      : /(\/\/[^\n]*)/g;

    /* Comments and strings are finished once tinted, so they are lifted out
       behind a placeholder rather than left in place. A tinted region left in
       place is still *text* to the number and keyword passes below, and their
       spans nest inside it -- the inner colour wins, so `1124` in a comment
       renders in the number colour and every `for`, `in`, `as` and `type` in
       English prose renders as a keyword. The placeholder is a private-use
       character between NULs: no digit and no word character, so neither pass
       can match it. */
    var held = [];
    function hold(html) {
      held.push(html);
      return "\u0000" + String.fromCharCode(0xe000 + held.length - 1) + "\u0000";
    }

    out = out.replace(/(&quot;|")([^"\n]*)(&quot;|")/g, function (m, a, b, c) {
      return hold('<span class="tok-str">' + a + b + c + '</span>');
    });
    /* The two regexes capture different numbers of groups -- the hash form
       keeps the newline before the comment, the `//` form has nothing to keep
       -- so the second argument is a capture in one case and `replace`'s
       offset argument in the other. Test what it *is*, not whether it is
       there: `b === undefined` is never true for the `//` form, which sent
       every Rust comment down the two-group branch and printed the offset
       into the page as a grey number at the end of the line. */
    out = out.replace(commentRe, function (m, a, b) {
      return (typeof b === "string") ? a + hold('<span class="tok-com">' + b + '</span>')
                                     : hold('<span class="tok-com">' + m + '</span>');
    });
    /* Every pass from here on runs only on the text between tags. Holding the
       spans aside already keeps them out of reach, so this is now the second
       of two guards -- kept because it is what the first failure needed:
       `class` is a Python keyword, and matching it inside an inserted
       attribute produced `<span <span class="tok-key">class</span>="tok-str">`,
       which a browser renders as a stray `="tok-str">` mid-code. Any Python
       block containing a string hit that. */
    out = outsideTags(out, /\b(\d+\.?\d*)\b/g, '<span class="tok-num">$1</span>');
    (KEYWORDS[lang] || []).forEach(function (kw) {
      out = outsideTags(out, new RegExp("\\b" + kw + "\\b", "g"),
                        '<span class="tok-key">' + kw + "</span>");
    });
    /* Restoring is a loop, not one pass: a string inside a comment is held
       first, so the comment's own held HTML contains that placeholder, and a
       single pass would put the comment back and leave the string's marker
       sitting in the page as text. */
    var placeholder = /\u0000([\ue000-\uf8ff])\u0000/g;
    while (placeholder.test(out)) {
      placeholder.lastIndex = 0;
      out = out.replace(placeholder, function (m, slot) {
        return held[slot.charCodeAt(0) - 0xe000];
      });
    }
    return out;
  }

  function decorateCode() {
    var pres = document.querySelectorAll("pre");
    Array.prototype.forEach.call(pres, function (pre) {
      var code = pre.querySelector("code");
      if (!code) return;

      var wrap = document.createElement("div");
      wrap.className = "codeblock";
      pre.parentNode.insertBefore(wrap, pre);
      wrap.appendChild(pre);

      var lang = (code.className.match(/language-([\w-]+)/) || [])[1];
      var raw = code.textContent;

      if (lang) {
        var tag = document.createElement("span");
        tag.className = "lang";
        tag.textContent = lang;
        wrap.appendChild(tag);
        if (!code.hasAttribute("data-no-tint")) {
          code.innerHTML = tint(raw, lang);
        }
      }

      var btn = document.createElement("button");
      btn.className = "copy-btn";
      btn.type = "button";
      btn.textContent = "copy";
      var copyKind = lang || "text";
      var copyLabel = "Copy " + copyKind + " code block";
      btn.setAttribute("aria-label", copyLabel);
      /* The label is the status, so it has to be announced -- a silent failure
         looks identical to a success to anyone not watching the button. */
      btn.setAttribute("aria-live", "polite");
      btn.addEventListener("click", function () {
        var reset = function () {
          setTimeout(function () {
            btn.textContent = "copy";
            btn.setAttribute("aria-label", copyLabel);
          }, 1200);
        };
        var done = function () {
          btn.textContent = "copied";
          btn.setAttribute("aria-label", "Copied " + copyKind + " code block");
          reset();
        };
        var failed = function () {
          btn.textContent = "copy failed";
          btn.setAttribute("aria-label", "Could not copy " + copyKind + " code block");
          reset();
        };
        if (navigator.clipboard) {
          navigator.clipboard.writeText(raw).then(done, failed);
        } else {
          var ta = document.createElement("textarea");
          ta.value = raw;
          document.body.appendChild(ta);
          ta.select();
          try {
            if (document.execCommand("copy")) { done(); } else { failed(); }
          } catch (e) { failed(); }
          document.body.removeChild(ta);
        }
      });
      wrap.appendChild(btn);
    });
  }

  /* ---------- mobile nav ------------------------------------------------ */

  function initNavToggle(sidebar, beforeNode) {
    var btn = document.createElement("button");
    btn.className = "nav-toggle";
    btn.type = "button";
    btn.setAttribute("aria-label", "Toggle navigation");
    btn.setAttribute("aria-controls", "sidebar");
    btn.setAttribute("aria-expanded", "false");
    var mobileNav = window.matchMedia("(max-width: 820px)");

    /* One place that sets the class and the attribute together, so the two
       cannot disagree about whether the menu is open. */
    function setOpen(open, moveFocus) {
      open = mobileNav.matches && open;
      document.body.classList.toggle("nav-open", open);
      btn.setAttribute("aria-expanded", open ? "true" : "false");
      btn.setAttribute("aria-label", open ? "Close navigation" : "Open navigation");
      btn.textContent = open ? "× Close" : "☰ Menu";
      if (mobileNav.matches) {
        sidebar.inert = !open;
        sidebar.setAttribute("aria-hidden", open ? "false" : "true");
      } else {
        sidebar.inert = false;
        sidebar.removeAttribute("aria-hidden");
      }
      if (moveFocus && open) {
        var first = sidebar.querySelector("input, button, a[href]");
        if (first) first.focus();
      } else if (moveFocus && !open) {
        btn.focus();
      }
    }

    btn.addEventListener("click", function () {
      setOpen(!document.body.classList.contains("nav-open"), true);
    });
    Array.prototype.forEach.call(document.querySelectorAll(".sidebar a"), function (link) {
      link.addEventListener("click", function () { setOpen(false); });
    });
    document.body.insertBefore(btn, beforeNode);
    setOpen(false, false);
    var syncNav = function () { setOpen(false, false); };
    if (mobileNav.addEventListener) mobileNav.addEventListener("change", syncNav);
    else mobileNav.addListener(syncNav);
    document.addEventListener("click", function (e) {
      if (!document.body.classList.contains("nav-open")) return;
      if (e.target.closest && (e.target.closest(".sidebar") || e.target.closest(".nav-toggle"))) return;
      setOpen(false, true);
    });
    document.addEventListener("keydown", function (e) {
      if (!document.body.classList.contains("nav-open")) return;
      if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false, true);
        return;
      }
      if (e.key !== "Tab") return;
      var focusable = [btn].concat(Array.prototype.slice.call(
        sidebar.querySelectorAll('a[href]:not([hidden]), button:not([disabled]), input:not([disabled])')
      )).filter(function (el) { return !el.hidden && el.offsetParent !== null; });
      if (!focusable.length) return;
      var first = focusable[0];
      var last = focusable[focusable.length - 1];
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    });
  }

  /* ---------- assemble --------------------------------------------------- */

  function boot() {
    var main = document.querySelector("main");
    if (!main) return;

    setTheme(initialTheme());
    main.id = "main-content";
    main.setAttribute("tabindex", "-1");
    var skip = document.createElement("a");
    skip.className = "skip-link";
    skip.href = "#main-content";
    skip.textContent = "Skip to main content";
    document.body.insertBefore(skip, document.body.firstChild);

    var content = document.createElement("div");
    content.className = "content";
    main.parentNode.insertBefore(content, main);
    content.appendChild(main);

    var layout = document.createElement("div");
    layout.className = "layout";
    content.parentNode.insertBefore(layout, content);
    var sidebar = buildSidebar();
    layout.appendChild(sidebar);
    layout.appendChild(content);

    /* Navigation is essential on small screens, so initialize its toggle before
       optional page enhancements such as the TOC, pager, and code decoration. */
    initNavToggle(sidebar, layout);

    var toc = buildToc(main);
    if (toc) { content.insertBefore(toc, main); trackToc(toc, main); }
    else { content.classList.add("no-toc"); }

    var pager = buildPager();
    if (pager) main.appendChild(pager);

    decorateCode();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
