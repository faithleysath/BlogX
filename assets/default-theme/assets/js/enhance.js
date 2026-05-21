document.addEventListener("DOMContentLoaded", () => {
  const showToast = (message, type = "success") => {
    let stack = document.querySelector(".toast-stack");
    if (!stack) {
      stack = document.createElement("div");
      stack.className = "toast-stack";
      stack.setAttribute("aria-live", "polite");
      stack.setAttribute("aria-atomic", "true");
      document.body.append(stack);
    }

    const toast = document.createElement("div");
    toast.className = `toast toast-${type}`;
    toast.textContent = message;
    stack.append(toast);

    window.setTimeout(() => {
      toast.classList.add("toast-leaving");
      window.setTimeout(() => toast.remove(), 240);
    }, 2400);
  };

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
      try {
        await navigator.clipboard.writeText(code.innerText);
        button.classList.add("copied");
        showToast("复制成功");
        window.setTimeout(() => {
          button.classList.remove("copied");
        }, 1200);
      } catch {
        showToast("复制失败", "error");
      }
    });
    block.prepend(button);
  }

  const images = Array.from(document.querySelectorAll("article img"));
  if (images.length > 0) {
    const lightbox = document.createElement("div");
    lightbox.className = "image-lightbox";
    lightbox.hidden = true;
    lightbox.innerHTML = `
      <button class="image-lightbox-close" type="button" aria-label="Close image">&times;</button>
      <button class="image-lightbox-nav image-lightbox-prev" type="button" aria-label="Previous image">&lsaquo;</button>
      <figure>
        <img alt="">
        <figcaption></figcaption>
      </figure>
      <button class="image-lightbox-nav image-lightbox-next" type="button" aria-label="Next image">&rsaquo;</button>
    `;
    document.body.append(lightbox);

    const lightboxImage = lightbox.querySelector("img");
    const caption = lightbox.querySelector("figcaption");
    const closeButton = lightbox.querySelector(".image-lightbox-close");
    const previousButton = lightbox.querySelector(".image-lightbox-prev");
    const nextButton = lightbox.querySelector(".image-lightbox-next");
    let currentIndex = 0;

    const openLightbox = index => {
      currentIndex = index;
      const image = images[currentIndex];
      lightboxImage.src = image.currentSrc || image.src;
      lightboxImage.alt = image.alt || "";
      caption.textContent = image.alt || "";
      caption.hidden = !image.alt;
      lightbox.hidden = false;
      document.body.classList.add("lightbox-open");
      closeButton.focus();
    };

    const closeLightbox = () => {
      lightbox.hidden = true;
      document.body.classList.remove("lightbox-open");
      lightboxImage.removeAttribute("src");
    };

    const moveLightbox = step => {
      currentIndex = (currentIndex + step + images.length) % images.length;
      openLightbox(currentIndex);
    };

    images.forEach((image, index) => {
      image.addEventListener("click", () => {
        if (image.naturalWidth < 100 || image.naturalHeight < 100) return;
        openLightbox(index);
      });
    });
    closeButton.addEventListener("click", closeLightbox);
    previousButton.addEventListener("click", () => moveLightbox(-1));
    nextButton.addEventListener("click", () => moveLightbox(1));
    lightbox.addEventListener("click", event => {
      if (event.target === lightbox) closeLightbox();
    });
    document.addEventListener("keydown", event => {
      if (lightbox.hidden) return;
      if (event.key === "Escape") closeLightbox();
      if (event.key === "ArrowLeft") moveLightbox(-1);
      if (event.key === "ArrowRight") moveLightbox(1);
    });
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
