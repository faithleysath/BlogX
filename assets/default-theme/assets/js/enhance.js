document.addEventListener("DOMContentLoaded", () => {
  for (const link of document.links) {
    if (link.origin !== window.location.origin) {
      link.target = "_blank";
      link.rel = "noopener noreferrer";
    }
  }

  for (const code of document.querySelectorAll("pre > code")) {
    const block = code.parentElement;
    if (!block) continue;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "copy-code";
    button.setAttribute("aria-label", "Copy code");
    button.title = "Copy code";
    button.addEventListener("click", async () => {
      await navigator.clipboard.writeText(code.innerText);
      button.classList.add("copied");
      window.setTimeout(() => {
        button.classList.remove("copied");
      }, 1200);
    });
    block.prepend(button);
  }

  for (const katexDisplay of document.querySelectorAll(".katex-display")) {
    katexDisplay.addEventListener(
      "wheel",
      event => {
        if (katexDisplay.scrollWidth > katexDisplay.clientWidth) {
          event.preventDefault();
          katexDisplay.scrollLeft += event.deltaY;
        }
      },
      { passive: false },
    );
  }
});
