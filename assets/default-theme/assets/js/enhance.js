document.addEventListener("DOMContentLoaded", () => {
  for (const code of document.querySelectorAll("pre > code")) {
    const block = code.parentElement;
    if (!block) continue;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "copy-code";
    button.textContent = "Copy";
    button.addEventListener("click", async () => {
      await navigator.clipboard.writeText(code.innerText);
      button.textContent = "Copied";
      window.setTimeout(() => {
        button.textContent = "Copy";
      }, 1200);
    });
    block.prepend(button);
  }
});
