// Minimal client-side glue. All state lives on the server; this only reacts to
// HX-Trigger events sent back by handlers.
document.body.addEventListener("task-created", () => {
  const toast = document.getElementById("toast");
  toast.textContent = "Task added";
  toast.hidden = false;
  clearTimeout(toast._t);
  toast._t = setTimeout(() => { toast.hidden = true; }, 1500);
});

// Surface server errors (e.g. 403 CSRF) instead of silently doing nothing.
document.body.addEventListener("htmx:responseError", (e) => {
  const status = e.detail.xhr.status;
  if (status === 422) return; // handled by swapping the form back in
  alert(`Request failed (${status}). Reload the page and try again.`);
});

// htmx does not swap 4xx responses by default; allow 422 so the form re-renders.
document.body.addEventListener("htmx:beforeSwap", (e) => {
  if (e.detail.xhr.status === 422) {
    e.detail.shouldSwap = true;
    e.detail.target = document.getElementById("task-form");
    e.detail.swapOverride = "outerHTML";
  }
});
