import { a as attr } from "../../../chunks/attributes.js";
import { e as escape_html } from "../../../chunks/escaping.js";
import "../../../chunks/auth.js";
import "@sveltejs/kit/internal";
import "../../../chunks/exports.js";
import "../../../chunks/utils.js";
import "@sveltejs/kit/internal/server";
import "../../../chunks/state.svelte.js";
function _page($$renderer, $$props) {
  $$renderer.component(($$renderer2) => {
    let token = "";
    let isLoading = false;
    $$renderer2.push(`<div class="login-container svelte-1x05zx6"><div class="login-card svelte-1x05zx6"><h1 class="svelte-1x05zx6">Dashboard Login</h1> <p class="subtitle svelte-1x05zx6">Enter your dashboard token to continue</p> <form><div class="form-group svelte-1x05zx6"><label for="token" class="svelte-1x05zx6">Token</label> <input id="token" type="password"${attr("value", token)} placeholder="Enter your token here"${attr("disabled", isLoading, true)} autocomplete="off" class="svelte-1x05zx6"/></div> `);
    {
      $$renderer2.push("<!--[!-->");
    }
    $$renderer2.push(`<!--]--> <button type="submit"${attr("disabled", isLoading, true)} class="svelte-1x05zx6">${escape_html("Login")}</button></form> <div class="help-text svelte-1x05zx6"><p class="svelte-1x05zx6">Don't have a token?</p> <p class="svelte-1x05zx6">Use the <code class="svelte-1x05zx6">/dashboard create-token</code> command in Discord to create one.</p></div></div></div>`);
  });
}
export {
  _page as default
};
