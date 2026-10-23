'use strict';
// The artwork is illustrative. No device discovery, permissions or telemetry.
(() => {
  const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
  const button = document.querySelector('.motion-toggle');
  let paused = reduced.matches;
  button.hidden = false;
  function apply() {
    document.body.classList.toggle('motion-paused', paused);
    button.setAttribute('aria-pressed', String(paused));
    button.disabled = reduced.matches;
    button.setAttribute('aria-label', reduced.matches ? 'Reduced motion enabled' : paused ? 'Resume animation' : 'Pause animation');
    button.textContent = reduced.matches ? 'Reduced motion' : paused ? 'Resume motion' : 'Pause motion';
  }
  button.addEventListener('click', () => { paused = !paused; apply(); });
  reduced.addEventListener('change', () => { paused = reduced.matches; apply(); });
  apply();
  if (!reduced.matches) document.body.classList.add('page-enter');
  // Suspend animation when the page is hidden; no background drawing loop.
  document.addEventListener('visibilitychange', () => {
    document.body.classList.toggle('motion-paused', paused || document.hidden);
  });
})();
