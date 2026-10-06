// The "Copy" button beside each command, as on flo2.io (flo2's site/copy.js).
// The page works without it: a button stays hidden unless the browser can write
// to the clipboard.
for (const button of document.querySelectorAll('button[data-copy]')) {
  if (!navigator.clipboard) continue;
  const source = document.getElementById(button.dataset.copy);
  button.hidden = false;
  button.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(source.textContent.trim());
      button.textContent = 'Copied';
    } catch {
      button.textContent = 'Select and copy';
    }
    setTimeout(() => (button.textContent = 'Copy'), 2000);
  });
}
