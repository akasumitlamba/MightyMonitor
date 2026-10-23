"use strict";
// Window-transfer choreography: a window travels between the illustrated screens.
// No device access, render loop, third-party runtime or network dependency.
(() => {
  const stage = document.querySelector(".artwork-stage");
  const pane = stage.querySelector(".travelling-window");
  const pointer = stage.querySelector(".demo-pointer");
  let mode = "mirror";
  let visible = true;
  let animations = [];
  const paths = {
    mirror: [
      { left: "20%", top: "25%", transform: "scale(.8) rotate(-5deg)" },
      {
        left: "63%",
        top: "38%",
        transform: "scale(1) rotate(5deg)",
        offset: 0.45,
      },
      {
        left: "63%",
        top: "38%",
        transform: "scale(1) rotate(5deg)",
        offset: 0.7,
      },
      { left: "20%", top: "25%", transform: "scale(.8) rotate(-5deg)" },
    ],
    control: [
      { left: "58%", top: "28%", transform: "scale(1)" },
      { left: "43%", top: "50%", transform: "scale(.65)", offset: 0.45 },
      { left: "43%", top: "50%", transform: "scale(.62)", offset: 0.55 },
      { left: "58%", top: "28%", transform: "scale(1)" },
    ],
    extend: [
      { left: "15%", top: "28%", transform: "rotate(-6deg)" },
      { left: "65%", top: "28%", transform: "rotate(5deg)", offset: 0.55 },
      { left: "65%", top: "28%", transform: "rotate(5deg)", offset: 0.75 },
      { left: "15%", top: "28%", transform: "rotate(-6deg)" },
    ],
  };
  function sync() {
    const paused =
      document.body.classList.contains("motion-paused") ||
      !visible ||
      document.hidden;
    for (const animation of animations)
      paused ? animation.pause() : animation.play();
  }
  function build() {
    animations.forEach((a) => a.cancel());
    animations = [
      pane.animate(paths[mode], {
        duration: 9000,
        iterations: Infinity,
        easing: "ease-in-out",
      }),
    ];
    pointer.hidden = mode !== "control";
    if (mode === "control")
      animations.push(
        pointer.animate(
          [
            { left: "72%", top: "42%" },
            { left: "50%", top: "66%", offset: 0.45 },
            { left: "50%", top: "66%", offset: 0.6 },
            { left: "72%", top: "42%" },
          ],
          { duration: 9000, iterations: Infinity, easing: "ease-in-out" },
        ),
      );
    sync();
  }
  window.addEventListener("mm-preview-mode", (e) => {
    if (paths[e.detail]) {
      mode = e.detail;
      build();
    }
  });
  new MutationObserver(sync).observe(document.body, {
    attributes: true,
    attributeFilter: ["class"],
  });
  document.addEventListener("visibilitychange", sync);
  if ("IntersectionObserver" in window)
    new IntersectionObserver((entries) => {
      visible = entries[0].isIntersecting;
      sync();
    }).observe(stage);
  build();
})();
