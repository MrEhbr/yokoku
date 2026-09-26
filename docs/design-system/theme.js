(() => {
  const key = 'yokoku-theme';
  const media = matchMedia('(prefers-color-scheme: dark)');
  let preference = 'system';
  try {
    const stored = localStorage.getItem(key);
    if (['light', 'dark', 'system'].includes(stored)) preference = stored;
  } catch {}
  function apply() {
    document.documentElement.dataset.theme = preference === 'system' ? (media.matches ? 'dark' : 'light') : preference;
    document.querySelectorAll('[data-theme-picker]').forEach(el => el.value = preference);
  }
  apply();
  media.addEventListener('change', apply);
  document.addEventListener('DOMContentLoaded', apply);
  document.addEventListener('change', event => {
    if (!event.target.matches('[data-theme-picker]')) return;
    const next = event.target.value;
    if (!['light', 'dark', 'system'].includes(next)) return;
    preference = next;
    try { localStorage.setItem(key, preference); } catch {}
    apply();
  });
})();
