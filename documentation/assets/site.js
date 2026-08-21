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
        ["index.html",                        "Overview"]
      ]
    },
    {
      title: "How It Works",
      items: [
        ["guide/pipeline.html",               "The Pipeline"],
        ["guide/variation.html",              "Variation & Selection"],
        ["guide/fitness.html",                "Fitness & Output"]
      ]
    },
    {
      title: "Using GET",
      items: [
        ["guide/route-python-objects.html",   "Python: Config Objects"],
        ["guide/route-python-toml.html",      "Python: TOML File"],
        ["guide/route-rust-library.html",     "Rust: As a Library"],
        ["guide/route-rust-cli.html",         "Rust: The get-run CLI"]
      ]
    },
    {
      title: "Extending GET",
      items: [
        ["guide/extending.html",              "Extension Points"],
        ["guide/new-fitness.html",            "Add an Objective"],
        ["guide/new-genome.html",             "Add a Genome"],
        ["guide/new-evolver.html",            "Add a Strategy"],
        ["guide/new-selection.html",          "Add a Selection Scheme"],
        ["guide/new-crossover.html",          "Add a Crossover Operator"],
        ["guide/new-mutation.html",           "Add a Mutation Operator"]
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
      '<a href="' + href("index.html") + '">GET docs</a>' +
      '<span class="tag">v0.9</span>';
    aside.appendChild(brand);

    /* One section open at a time: the one holding the current page. A title is
       a link to its own first page, so following it lands you there and the
       next page load opens that section and closes this one -- the accordion
       needs no state of its own, and a shared link always arrives showing the
       same thing. */
    var nav = document.createElement("nav");
    NAV.forEach(function (group) {
      var holdsPage = group.items.some(function (i) { return i[0] === page; });

      var g = document.createElement("div");
      // A titleless group has nothing to collapse, so it is always open.
      g.className = "nav-group" + (holdsPage || !group.title ? " open" : "");

      if (group.title) {
        var t = document.createElement("a");
        t.className = "nav-title";
        t.href = href(group.items[0][0]);
        t.textContent = group.title;
        t.setAttribute("aria-expanded", holdsPage ? "true" : "false");
        g.appendChild(t);
      }

      var list = document.createElement("div");
      list.className = "nav-items";
      group.items.forEach(function (item) {
        var a = document.createElement("a");
        a.className = "nav-link" + (item[0] === page ? " active" : "");
        a.href = href(item[0]);
        a.textContent = item[1];
        list.appendChild(a);
      });
      g.appendChild(list);
      nav.appendChild(g);
    });
    aside.appendChild(nav);
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
      if (!h.closest(".card")) heads.push(h);
    });
    if (heads.length < 3) return null;

    var toc = document.createElement("nav");
    toc.className = "toc";
    var title = document.createElement("div");
    title.className = "toc-title";
    title.textContent = "On This Page";
    toc.appendChild(title);

    Array.prototype.forEach.call(heads, function (h) {
      if (!h.id) h.id = slugify(h.textContent);
      var a = document.createElement("a");
      a.href = "#" + h.id;
      a.textContent = h.textContent.replace(/\s*#\s*$/, "");
      a.className = h.tagName === "H3" ? "lvl-3" : "lvl-2";
      toc.appendChild(a);

      var link = document.createElement("a");
      link.className = "anchor";
      link.href = "#" + h.id;
      link.textContent = "#";
      link.setAttribute("aria-hidden", "true");
      h.appendChild(link);
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

    out = out.replace(/(&quot;|")([^"\n]*)(&quot;|")/g, '<span class="tok-str">$1$2$3</span>');
    out = out.replace(commentRe, function (m, a, b) {
      return (b === undefined) ? '<span class="tok-com">' + m + '</span>'
                               : a + '<span class="tok-com">' + b + '</span>';
    });
    /* Every pass from here on runs only on the text between tags. The string
       and comment passes above have already inserted `class="tok-..."`
       attributes, and `class` is a Python keyword -- matching it inside an
       attribute produced `<span <span class="tok-key">class</span>="tok-str">`,
       which a browser renders as a stray `="tok-str">` in the middle of the
       code. Any Python block containing a string hit this. */
    out = outsideTags(out, /\b(\d+\.?\d*)\b/g, '<span class="tok-num">$1</span>');
    (KEYWORDS[lang] || []).forEach(function (kw) {
      out = outsideTags(out, new RegExp("\\b" + kw + "\\b", "g"),
                        '<span class="tok-key">' + kw + "</span>");
    });
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
      btn.addEventListener("click", function () {
        var done = function () {
          btn.textContent = "copied";
          setTimeout(function () { btn.textContent = "copy"; }, 1200);
        };
        if (navigator.clipboard) {
          navigator.clipboard.writeText(raw).then(done, function () {});
        } else {
          var ta = document.createElement("textarea");
          ta.value = raw;
          document.body.appendChild(ta);
          ta.select();
          try { document.execCommand("copy"); done(); } catch (e) {}
          document.body.removeChild(ta);
        }
      });
      wrap.appendChild(btn);
    });
  }

  /* ---------- mobile nav ------------------------------------------------ */

  function initNavToggle() {
    var btn = document.createElement("button");
    btn.className = "nav-toggle";
    btn.type = "button";
    btn.setAttribute("aria-label", "Toggle navigation");
    btn.textContent = "☰";
    btn.addEventListener("click", function () {
      document.body.classList.toggle("nav-open");
    });
    document.body.appendChild(btn);
    document.addEventListener("click", function (e) {
      if (!document.body.classList.contains("nav-open")) return;
      if (e.target.closest && (e.target.closest(".sidebar") || e.target.closest(".nav-toggle"))) return;
      document.body.classList.remove("nav-open");
    });
  }

  /* ---------- assemble --------------------------------------------------- */

  function boot() {
    var main = document.querySelector("main");
    if (!main) return;

    var content = document.createElement("div");
    content.className = "content";
    main.parentNode.insertBefore(content, main);
    content.appendChild(main);

    var layout = document.createElement("div");
    layout.className = "layout";
    content.parentNode.insertBefore(layout, content);
    layout.appendChild(buildSidebar());
    layout.appendChild(content);

    var toc = buildToc(main);
    if (toc) { content.appendChild(toc); trackToc(toc, main); }
    else { content.classList.add("no-toc"); }

    var pager = buildPager();
    if (pager) main.appendChild(pager);

    decorateCode();
    initNavToggle();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
