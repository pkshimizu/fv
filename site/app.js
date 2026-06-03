/* ============================================================
   fv landing page — interactions
   - language toggle (EN default, persisted in localStorage)
   - sticky header state
   - scroll-reveal (respects prefers-reduced-motion)
   - copy-to-clipboard for install commands
   ============================================================ */
(function () {
  'use strict';

  /* ---------- i18n ---------- */
  var STORAGE_KEY = 'fv-lang';
  var SUPPORTED = ['en', 'ja'];

  function getInitialLang() {
    var saved = null;
    try { saved = localStorage.getItem(STORAGE_KEY); } catch (e) {}
    if (saved && SUPPORTED.indexOf(saved) !== -1) return saved;
    return 'en'; // default English
  }

  function applyLang(lang) {
    if (SUPPORTED.indexOf(lang) === -1) lang = 'en';

    document.documentElement.lang = lang;
    document.title = lang === 'ja'
      ? 'fv — ターミナルで完結する軽快なファイルマネージャ'
      : 'fv — a fast, keyboard-driven terminal file manager';

    var nodes = document.querySelectorAll('[data-en]');
    for (var i = 0; i < nodes.length; i++) {
      var el = nodes[i];
      var val = el.getAttribute('data-' + lang);
      if (val != null) el.textContent = val;
    }

    // toggle active state on buttons
    var btns = document.querySelectorAll('.lang-toggle button');
    for (var j = 0; j < btns.length; j++) {
      btns[j].classList.toggle('active', btns[j].getAttribute('data-lang') === lang);
    }

    try { localStorage.setItem(STORAGE_KEY, lang); } catch (e) {}
    // let other scripts know (tweaks panel re-applies its own copy after lang change)
    window.dispatchEvent(new CustomEvent('fv:langchange', { detail: { lang: lang } }));
  }

  document.addEventListener('click', function (e) {
    var btn = e.target.closest && e.target.closest('.lang-toggle button');
    if (!btn) return;
    applyLang(btn.getAttribute('data-lang'));
  });

  applyLang(getInitialLang());
  window.fvApplyLang = applyLang;

  /* ---------- sticky header ---------- */
  var header = document.getElementById('siteHeader');
  function onScroll() {
    if (!header) return;
    header.classList.toggle('scrolled', window.scrollY > 8);
  }
  onScroll();
  window.addEventListener('scroll', onScroll, { passive: true });

  /* ---------- scroll reveal ---------- */
  var reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var revealEls = document.querySelectorAll('.reveal');

  // Enable the hidden start-state only now that JS is running.
  document.documentElement.classList.add('js-reveal');

  function revealAll() {
    for (var r = 0; r < revealEls.length; r++) revealEls[r].classList.add('in');
  }

  if (reduceMotion || !('IntersectionObserver' in window)) {
    revealAll();
  } else {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add('in');
          io.unobserve(entry.target);
        }
      });
    }, { threshold: 0.12, rootMargin: '0px 0px -8% 0px' });
    for (var k = 0; k < revealEls.length; k++) io.observe(revealEls[k]);

    // Safety net: some rendering contexts (offscreen/headless iframes) never
    // fire IO. Guarantee content is visible regardless after a short delay.
    setTimeout(revealAll, 1300);
  }

  /* ---------- copy to clipboard ---------- */
  document.addEventListener('click', function (e) {
    var btn = e.target.closest && e.target.closest('.copy-btn');
    if (!btn) return;
    var block = btn.closest('.code-block');
    if (!block) return;
    var text = (block.getAttribute('data-copy') || '').replace(/&#10;/g, '\n');

    var done = function () {
      var lang = document.documentElement.lang === 'ja' ? 'ja' : 'en';
      btn.classList.add('copied');
      btn.textContent = lang === 'ja' ? 'コピーしました' : 'Copied';
      setTimeout(function () {
        btn.classList.remove('copied');
        btn.textContent = btn.getAttribute('data-' + lang) || 'Copy';
      }, 1600);
    };

    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done).catch(done);
    } else {
      var ta = document.createElement('textarea');
      ta.value = text; document.body.appendChild(ta); ta.select();
      try { document.execCommand('copy'); } catch (err) {}
      document.body.removeChild(ta);
      done();
    }
  });
})();
