"use strict";

function changePage(initialLoad) {
  let page;
  const hash = location.hash;
  const id = hash.startsWith("#") ? hash.substr(1) : null;
  let targetEl = null;
  if (id) {
    targetEl = document.getElementById(id);
    page = targetEl?.closest(".olm-page");
  }
  if (!page) {
    if (!initialLoad) {
      // Make no change.
      return;
    }
    page = document.getElementById("index");
  }
  const currPage = document.querySelector(".olm-page:not(.d-none)");
  if (currPage === page) {
    // No change in page.
    return;
  }

  currPage?.classList.add("d-none");
  page.classList.remove("d-none");

  // If we are targeting an element that is not a page element, we want to
  // scroll it into view (the focus position should have already shifted to
  // point to it).
  // Otherwise, if we have no target or the target is a page, we want to scroll
  // to the top of the document so that the header is visible.
  const doScroll = () => {
    const scrollEl = targetEl && targetEl !== page ? targetEl : document.body;
    scrollEl.scrollIntoView({
      behavior: "instant",
      block: "start",
      inline: "start",
    });
  };
  if (initialLoad) {
    window.addEventListener("load", doScroll, { once: true });
  } else {
    doScroll();
  }
}

window.addEventListener("DOMContentLoaded", () => {
  const langDropdown = document.getElementById("language-menu-dropdown");
  for (const langButton of langDropdown.querySelectorAll("button")) {
    langButton.addEventListener("click", () => {
      dispatchEvent(
        new CustomEvent("ManualLocaleSelected", {
          // The button's lang attribute is expected to match the locale's code.
          detail: langButton.getAttribute("lang"),
          bubbles: true,
        })
      );
    });
  }

  changePage(true);
});

window.addEventListener("hashchange", () => {
  changePage(false);
});
