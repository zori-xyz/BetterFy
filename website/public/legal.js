// Language and theme for the static pages. Mirrors the keys the main site uses,
// so a choice made there carries over. Every storage access is guarded.
(function () {
  var root = document.documentElement;
  function read(key) { try { return localStorage.getItem(key); } catch (e) { return null; } }
  function write(key, value) { try { localStorage.setItem(key, value); } catch (e) { /* storage denied */ } }

  var fromUrl = new URLSearchParams(location.search).get("lang");
  var lang = fromUrl === "en" || fromUrl === "ru" ? fromUrl : read("betterfy-site-language") === "en" ? "en" : "ru";
  var theme = read("betterfy-site-theme");
  if (theme !== "light" && theme !== "dark") {
    try { theme = matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark"; } catch (e) { theme = "dark"; }
  }
  root.dataset.theme = theme;

  function apply(next, remember) {
    root.lang = next;
    if (remember) write("betterfy-site-language", next);
    var title = document.querySelector("title");
    if (title) title.textContent = title.getAttribute("data-" + next) || title.textContent;
    Array.prototype.forEach.call(document.querySelectorAll("[data-set-lang]"), function (button) {
      button.setAttribute("aria-pressed", String(button.getAttribute("data-set-lang") === next));
    });
  }

  apply(lang, false);
  document.addEventListener("DOMContentLoaded", function () {
    apply(lang, false);
    Array.prototype.forEach.call(document.querySelectorAll("[data-set-lang]"), function (button) {
      button.addEventListener("click", function () { apply(button.getAttribute("data-set-lang"), true); });
    });
  });
})();
