"use strict";
// Mode selection changes the artwork; nothing floats across the devices.
(() => {
  const image = document.querySelector(".workspace-art");
  const scenes = {
    mirror: [
      "mirror",
      "Phone and laptop displaying the same green landscape. Concept artwork.",
    ],
    control: ["control", "A cursor reaching a phone screen. Concept artwork."],
    extend: [
      "extend",
      "One continuous workspace across two displays. Concept artwork.",
    ],
  };
  window.addEventListener("mm-preview-mode", ({ detail }) => {
    const scene = scenes[detail];
    if (!scene) return;
    image.src = `assets/artwork/${scene[0]}.webp`;
    image.alt = scene[1];
  });
})();
