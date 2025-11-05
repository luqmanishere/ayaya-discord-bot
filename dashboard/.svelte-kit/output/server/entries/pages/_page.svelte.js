import { V as store_get, W as unsubscribe_stores } from "../../chunks/index2.js";
import { a as auth } from "../../chunks/auth.js";
import { e as escape_html } from "../../chunks/escaping.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    var $$store_subs;
    $$renderer2.push(`<div class="dashboard svelte-1uha8ag"><header class="svelte-1uha8ag"><h1 class="svelte-1uha8ag">Ayaya Dashboard</h1> <div class="user-info svelte-1uha8ag"><span class="svelte-1uha8ag">User ID: ${escape_html(store_get($$store_subs ??= {}, "$auth", auth).userId)}</span> <button class="logout-btn svelte-1uha8ag">Logout</button></div></header> <main class="svelte-1uha8ag"><div class="welcome-card svelte-1uha8ag"><h2 class="svelte-1uha8ag">Welcome to the Dashboard</h2> <p class="svelte-1uha8ag">You are successfully authenticated.</p></div> <div class="info-section svelte-1uha8ag"><h3 class="svelte-1uha8ag">Getting Started</h3> <ul class="svelte-1uha8ag"><li class="svelte-1uha8ag">Your dashboard token is securely stored in this browser</li> <li class="svelte-1uha8ag">Use Discord commands to manage your tokens</li> <li class="svelte-1uha8ag"><code class="svelte-1uha8ag">/dashboard list-tokens</code> - View your active tokens</li> <li class="svelte-1uha8ag"><code class="svelte-1uha8ag">/dashboard revoke-my-token</code> - Revoke a token</li></ul></div></main></div>`);
    if ($$store_subs) unsubscribe_stores($$store_subs);
  });
}
export {
  _page as default
};
