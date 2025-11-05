
// this file is generated — do not edit it


/// <reference types="@sveltejs/kit" />

/**
 * Environment variables [loaded by Vite](https://vitejs.dev/guide/env-and-mode.html#env-files) from `.env` files and `process.env`. Like [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), this module cannot be imported into client-side code. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured).
 * 
 * _Unlike_ [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), the values exported from this module are statically injected into your bundle at build time, enabling optimisations like dead code elimination.
 * 
 * ```ts
 * import { API_KEY } from '$env/static/private';
 * ```
 * 
 * Note that all environment variables referenced in your code should be declared (for example in an `.env` file), even if they don't have a value until the app is deployed:
 * 
 * ```
 * MY_FEATURE_FLAG=""
 * ```
 * 
 * You can override `.env` values from the command line like so:
 * 
 * ```sh
 * MY_FEATURE_FLAG="enabled" npm run dev
 * ```
 */
declare module '$env/static/private' {
	export const SHELL: string;
	export const npm_command: string;
	export const COREPACK_ENABLE_AUTO_PIN: string;
	export const __ETC_PROFILE_DONE: string;
	export const npm_config_userconfig: string;
	export const COLORTERM: string;
	export const __HM_SESS_VARS_SOURCED: string;
	export const XDG_CONFIG_DIRS: string;
	export const npm_config_cache: string;
	export const _PYTHON_HOST_PLATFORM: string;
	export const NIX_BUILD_CORES: string;
	export const ALSA_CONFIG_UCM2: string;
	export const configureFlags: string;
	export const DATABASE_URL: string;
	export const MEOW: string;
	export const mesonFlags: string;
	export const PKG_CONFIG_PATH: string;
	export const PYTHONNOUSERSITE: string;
	export const shell: string;
	export const depsHostHost: string;
	export const NODE: string;
	export const PYTHONHASHSEED: string;
	export const VSCODE_PROCESS_TITLE: string;
	export const SSH_AUTH_SOCK: string;
	export const DIRENV_DIR: string;
	export const STRINGS: string;
	export const DISCORD_TOKEN: string;
	export const CMAKE_INCLUDE_PATH: string;
	export const XDG_CONFIG_HOME: string;
	export const depsTargetTarget: string;
	export const XCURSOR_PATH: string;
	export const MEMORY_PRESSURE_WRITE: string;
	export const ELECTRON_RUN_AS_NODE: string;
	export const stdenv: string;
	export const AGE_SECRET_KEY: string;
	export const COLOR: string;
	export const LOCALE_ARCHIVE_2_27: string;
	export const npm_config_local_prefix: string;
	export const builder: string;
	export const DESKTOP_SESSION: string;
	export const SSH_AGENT_PID: string;
	export const GDK_PIXBUF_MODULE_FILE: string;
	export const ELECTRON_OZONE_PLATFORM_HINT: string;
	export const NO_AT_BRIDGE: string;
	export const shellHook: string;
	export const NIX_CC_WRAPPER_TARGET_HOST_aarch64_unknown_linux_gnu: string;
	export const npm_config_globalconfig: string;
	export const XCURSOR_SIZE: string;
	export const DIRENV_FILE: string;
	export const EDITOR: string;
	export const phases: string;
	export const XDG_SEAT: string;
	export const PWD: string;
	export const NIX_PROFILES: string;
	export const SOURCE_DATE_EPOCH: string;
	export const LOGNAME: string;
	export const XDG_SESSION_DESKTOP: string;
	export const XDG_SESSION_TYPE: string;
	export const NIX_ENFORCE_NO_NATIVE: string;
	export const NIX_PKG_CONFIG_WRAPPER_TARGET_HOST_aarch64_unknown_linux_gnu: string;
	export const CUPS_DATADIR: string;
	export const NIX_PATH: string;
	export const VSCODE_ESM_ENTRYPOINT: string;
	export const GCAT: string;
	export const npm_config_init_module: string;
	export const GPG_AGENT_INFO: string;
	export const SYSTEMD_EXEC_PID: string;
	export const NIXPKGS_CONFIG: string;
	export const VSCODE_CODE_CACHE_PATH: string;
	export const CXX: string;
	export const _: string;
	export const system: string;
	export const NoDefaultCurrentDirectoryInExePath: string;
	export const _PYTHON_SYSCONFIGDATA_NAME: string;
	export const HOST_PATH: string;
	export const CLAUDECODE: string;
	export const IN_NIX_SHELL: string;
	export const doInstallCheck: string;
	export const HOME: string;
	export const NIX_BINTOOLS: string;
	export const CLAUDE_AGENT_SDK_VERSION: string;
	export const LANG: string;
	export const TMUX_TMPDIR: string;
	export const XDG_CURRENT_DESKTOP: string;
	export const depsTargetTargetPropagated: string;
	export const npm_package_version: string;
	export const MEMORY_PRESSURE_WATCH: string;
	export const VSCODE_IPC_HOOK: string;
	export const WAYLAND_DISPLAY: string;
	export const cmakeFlags: string;
	export const STARSHIP_CONFIG: string;
	export const GIO_EXTRA_MODULES: string;
	export const outputs: string;
	export const VSCODE_CLI: string;
	export const NIX_STORE: string;
	export const VSCODE_L10N_BUNDLE_LOCATION: string;
	export const LD: string;
	export const INVOCATION_ID: string;
	export const buildPhase: string;
	export const NIRI_SOCKET: string;
	export const MANAGERPID: string;
	export const DIRENV_DIFF: string;
	export const DIRENV_CONFIG: string;
	export const INIT_CWD: string;
	export const READELF: string;
	export const CHROME_DESKTOP: string;
	export const CURSOR_CLI_MODE: string;
	export const GTK_A11Y: string;
	export const NIX_USER_PROFILE_DIR: string;
	export const INFOPATH: string;
	export const npm_lifecycle_script: string;
	export const doCheck: string;
	export const npm_config_npm_version: string;
	export const depsBuildBuild: string;
	export const PYTHONPATH: string;
	export const TERM: string;
	export const npm_package_name: string;
	export const GTK_PATH: string;
	export const SIZE: string;
	export const propagatedNativeBuildInputs: string;
	export const npm_config_prefix: string;
	export const USER: string;
	export const strictDeps: string;
	export const TZDIR: string;
	export const AR: string;
	export const AS: string;
	export const DISPLAY: string;
	export const npm_lifecycle_event: string;
	export const VSCODE_PID: string;
	export const SHLVL: string;
	export const NM: string;
	export const GIT_EDITOR: string;
	export const PAGER: string;
	export const NIX_CFLAGS_COMPILE: string;
	export const QTWEBKIT_PLUGIN_PATH: string;
	export const patches: string;
	export const VSCODE_CWD: string;
	export const __NIXOS_SET_ENVIRONMENT_DONE: string;
	export const NIXPKGS_CMAKE_PREFIX_PATH: string;
	export const XDG_VTNR: string;
	export const buildInputs: string;
	export const XDG_SESSION_ID: string;
	export const LOCALE_ARCHIVE: string;
	export const preferLocalBuild: string;
	export const MANAGERPIDFDID: string;
	export const LESSKEYIN_SYSTEM: string;
	export const npm_config_user_agent: string;
	export const QML2_IMPORT_PATH: string;
	export const TERMINFO_DIRS: string;
	export const OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE: string;
	export const npm_execpath: string;
	export const LD_LIBRARY_PATH: string;
	export const VSCODE_CRASH_REPORTER_PROCESS_TYPE: string;
	export const XDG_RUNTIME_DIR: string;
	export const CURSOR_CLI: string;
	export const depsBuildTarget: string;
	export const CLAUDE_CODE_ENTRYPOINT: string;
	export const OBJCOPY: string;
	export const DETERMINISTIC_BUILD: string;
	export const NIX_XDG_DESKTOP_PORTAL_DIR: string;
	export const out: string;
	export const npm_package_json: string;
	export const ELECTRON_NO_ATTACH_CONSOLE: string;
	export const STRIP: string;
	export const JOURNAL_STREAM: string;
	export const XCURSOR_THEME: string;
	export const XDG_DATA_DIRS: string;
	export const GDK_BACKEND: string;
	export const LIBEXEC_PATH: string;
	export const OBJDUMP: string;
	export const npm_config_noproxy: string;
	export const PATH: string;
	export const propagatedBuildInputs: string;
	export const npm_config_node_gyp: string;
	export const dontAddDisableDepTrack: string;
	export const __fish_nixos_env_preinit_sourced: string;
	export const CC: string;
	export const NIX_CC: string;
	export const ORIGINAL_XDG_CURRENT_DESKTOP: string;
	export const NIX_BINTOOLS_WRAPPER_TARGET_HOST_aarch64_unknown_linux_gnu: string;
	export const DBUS_SESSION_BUS_ADDRESS: string;
	export const depsBuildTargetPropagated: string;
	export const FZF_DEFAULT_OPTS: string;
	export const depsBuildBuildPropagated: string;
	export const DIRENV_WATCHES: string;
	export const npm_config_global_prefix: string;
	export const VSCODE_NLS_CONFIG: string;
	export const QT_PLUGIN_PATH: string;
	export const CONFIG_SHELL: string;
	export const __structuredAttrs: string;
	export const npm_node_execpath: string;
	export const RANLIB: string;
	export const npm_config_engine_strict: string;
	export const NIX_HARDENING_ENABLE: string;
	export const VSCODE_HANDLES_UNCAUGHT_ERRORS: string;
	export const NIX_LDFLAGS: string;
	export const nativeBuildInputs: string;
	export const CMAKE_LIBRARY_PATH: string;
	export const name: string;
	export const PKG_CONFIG: string;
	export const CURSOR_TRACE_ID: string;
	export const depsHostHostPropagated: string;
	export const NODE_ENV: string;
}

/**
 * Similar to [`$env/static/private`](https://svelte.dev/docs/kit/$env-static-private), except that it only includes environment variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Values are replaced statically at build time.
 * 
 * ```ts
 * import { PUBLIC_BASE_URL } from '$env/static/public';
 * ```
 */
declare module '$env/static/public' {
	
}

/**
 * This module provides access to runtime environment variables, as defined by the platform you're running on. For example if you're using [`adapter-node`](https://github.com/sveltejs/kit/tree/main/packages/adapter-node) (or running [`vite preview`](https://svelte.dev/docs/kit/cli)), this is equivalent to `process.env`. This module only includes variables that _do not_ begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) _and do_ start with [`config.kit.env.privatePrefix`](https://svelte.dev/docs/kit/configuration#env) (if configured).
 * 
 * This module cannot be imported into client-side code.
 * 
 * ```ts
 * import { env } from '$env/dynamic/private';
 * console.log(env.DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 * 
 * > [!NOTE] In `dev`, `$env/dynamic` always includes environment variables from `.env`. In `prod`, this behavior will depend on your adapter.
 */
declare module '$env/dynamic/private' {
	export const env: {
		SHELL: string;
		npm_command: string;
		COREPACK_ENABLE_AUTO_PIN: string;
		__ETC_PROFILE_DONE: string;
		npm_config_userconfig: string;
		COLORTERM: string;
		__HM_SESS_VARS_SOURCED: string;
		XDG_CONFIG_DIRS: string;
		npm_config_cache: string;
		_PYTHON_HOST_PLATFORM: string;
		NIX_BUILD_CORES: string;
		ALSA_CONFIG_UCM2: string;
		configureFlags: string;
		DATABASE_URL: string;
		MEOW: string;
		mesonFlags: string;
		PKG_CONFIG_PATH: string;
		PYTHONNOUSERSITE: string;
		shell: string;
		depsHostHost: string;
		NODE: string;
		PYTHONHASHSEED: string;
		VSCODE_PROCESS_TITLE: string;
		SSH_AUTH_SOCK: string;
		DIRENV_DIR: string;
		STRINGS: string;
		DISCORD_TOKEN: string;
		CMAKE_INCLUDE_PATH: string;
		XDG_CONFIG_HOME: string;
		depsTargetTarget: string;
		XCURSOR_PATH: string;
		MEMORY_PRESSURE_WRITE: string;
		ELECTRON_RUN_AS_NODE: string;
		stdenv: string;
		AGE_SECRET_KEY: string;
		COLOR: string;
		LOCALE_ARCHIVE_2_27: string;
		npm_config_local_prefix: string;
		builder: string;
		DESKTOP_SESSION: string;
		SSH_AGENT_PID: string;
		GDK_PIXBUF_MODULE_FILE: string;
		ELECTRON_OZONE_PLATFORM_HINT: string;
		NO_AT_BRIDGE: string;
		shellHook: string;
		NIX_CC_WRAPPER_TARGET_HOST_aarch64_unknown_linux_gnu: string;
		npm_config_globalconfig: string;
		XCURSOR_SIZE: string;
		DIRENV_FILE: string;
		EDITOR: string;
		phases: string;
		XDG_SEAT: string;
		PWD: string;
		NIX_PROFILES: string;
		SOURCE_DATE_EPOCH: string;
		LOGNAME: string;
		XDG_SESSION_DESKTOP: string;
		XDG_SESSION_TYPE: string;
		NIX_ENFORCE_NO_NATIVE: string;
		NIX_PKG_CONFIG_WRAPPER_TARGET_HOST_aarch64_unknown_linux_gnu: string;
		CUPS_DATADIR: string;
		NIX_PATH: string;
		VSCODE_ESM_ENTRYPOINT: string;
		GCAT: string;
		npm_config_init_module: string;
		GPG_AGENT_INFO: string;
		SYSTEMD_EXEC_PID: string;
		NIXPKGS_CONFIG: string;
		VSCODE_CODE_CACHE_PATH: string;
		CXX: string;
		_: string;
		system: string;
		NoDefaultCurrentDirectoryInExePath: string;
		_PYTHON_SYSCONFIGDATA_NAME: string;
		HOST_PATH: string;
		CLAUDECODE: string;
		IN_NIX_SHELL: string;
		doInstallCheck: string;
		HOME: string;
		NIX_BINTOOLS: string;
		CLAUDE_AGENT_SDK_VERSION: string;
		LANG: string;
		TMUX_TMPDIR: string;
		XDG_CURRENT_DESKTOP: string;
		depsTargetTargetPropagated: string;
		npm_package_version: string;
		MEMORY_PRESSURE_WATCH: string;
		VSCODE_IPC_HOOK: string;
		WAYLAND_DISPLAY: string;
		cmakeFlags: string;
		STARSHIP_CONFIG: string;
		GIO_EXTRA_MODULES: string;
		outputs: string;
		VSCODE_CLI: string;
		NIX_STORE: string;
		VSCODE_L10N_BUNDLE_LOCATION: string;
		LD: string;
		INVOCATION_ID: string;
		buildPhase: string;
		NIRI_SOCKET: string;
		MANAGERPID: string;
		DIRENV_DIFF: string;
		DIRENV_CONFIG: string;
		INIT_CWD: string;
		READELF: string;
		CHROME_DESKTOP: string;
		CURSOR_CLI_MODE: string;
		GTK_A11Y: string;
		NIX_USER_PROFILE_DIR: string;
		INFOPATH: string;
		npm_lifecycle_script: string;
		doCheck: string;
		npm_config_npm_version: string;
		depsBuildBuild: string;
		PYTHONPATH: string;
		TERM: string;
		npm_package_name: string;
		GTK_PATH: string;
		SIZE: string;
		propagatedNativeBuildInputs: string;
		npm_config_prefix: string;
		USER: string;
		strictDeps: string;
		TZDIR: string;
		AR: string;
		AS: string;
		DISPLAY: string;
		npm_lifecycle_event: string;
		VSCODE_PID: string;
		SHLVL: string;
		NM: string;
		GIT_EDITOR: string;
		PAGER: string;
		NIX_CFLAGS_COMPILE: string;
		QTWEBKIT_PLUGIN_PATH: string;
		patches: string;
		VSCODE_CWD: string;
		__NIXOS_SET_ENVIRONMENT_DONE: string;
		NIXPKGS_CMAKE_PREFIX_PATH: string;
		XDG_VTNR: string;
		buildInputs: string;
		XDG_SESSION_ID: string;
		LOCALE_ARCHIVE: string;
		preferLocalBuild: string;
		MANAGERPIDFDID: string;
		LESSKEYIN_SYSTEM: string;
		npm_config_user_agent: string;
		QML2_IMPORT_PATH: string;
		TERMINFO_DIRS: string;
		OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE: string;
		npm_execpath: string;
		LD_LIBRARY_PATH: string;
		VSCODE_CRASH_REPORTER_PROCESS_TYPE: string;
		XDG_RUNTIME_DIR: string;
		CURSOR_CLI: string;
		depsBuildTarget: string;
		CLAUDE_CODE_ENTRYPOINT: string;
		OBJCOPY: string;
		DETERMINISTIC_BUILD: string;
		NIX_XDG_DESKTOP_PORTAL_DIR: string;
		out: string;
		npm_package_json: string;
		ELECTRON_NO_ATTACH_CONSOLE: string;
		STRIP: string;
		JOURNAL_STREAM: string;
		XCURSOR_THEME: string;
		XDG_DATA_DIRS: string;
		GDK_BACKEND: string;
		LIBEXEC_PATH: string;
		OBJDUMP: string;
		npm_config_noproxy: string;
		PATH: string;
		propagatedBuildInputs: string;
		npm_config_node_gyp: string;
		dontAddDisableDepTrack: string;
		__fish_nixos_env_preinit_sourced: string;
		CC: string;
		NIX_CC: string;
		ORIGINAL_XDG_CURRENT_DESKTOP: string;
		NIX_BINTOOLS_WRAPPER_TARGET_HOST_aarch64_unknown_linux_gnu: string;
		DBUS_SESSION_BUS_ADDRESS: string;
		depsBuildTargetPropagated: string;
		FZF_DEFAULT_OPTS: string;
		depsBuildBuildPropagated: string;
		DIRENV_WATCHES: string;
		npm_config_global_prefix: string;
		VSCODE_NLS_CONFIG: string;
		QT_PLUGIN_PATH: string;
		CONFIG_SHELL: string;
		__structuredAttrs: string;
		npm_node_execpath: string;
		RANLIB: string;
		npm_config_engine_strict: string;
		NIX_HARDENING_ENABLE: string;
		VSCODE_HANDLES_UNCAUGHT_ERRORS: string;
		NIX_LDFLAGS: string;
		nativeBuildInputs: string;
		CMAKE_LIBRARY_PATH: string;
		name: string;
		PKG_CONFIG: string;
		CURSOR_TRACE_ID: string;
		depsHostHostPropagated: string;
		NODE_ENV: string;
		[key: `PUBLIC_${string}`]: undefined;
		[key: `${string}`]: string | undefined;
	}
}

/**
 * Similar to [`$env/dynamic/private`](https://svelte.dev/docs/kit/$env-dynamic-private), but only includes variables that begin with [`config.kit.env.publicPrefix`](https://svelte.dev/docs/kit/configuration#env) (which defaults to `PUBLIC_`), and can therefore safely be exposed to client-side code.
 * 
 * Note that public dynamic environment variables must all be sent from the server to the client, causing larger network requests — when possible, use `$env/static/public` instead.
 * 
 * ```ts
 * import { env } from '$env/dynamic/public';
 * console.log(env.PUBLIC_DEPLOYMENT_SPECIFIC_VARIABLE);
 * ```
 */
declare module '$env/dynamic/public' {
	export const env: {
		[key: `PUBLIC_${string}`]: string | undefined;
	}
}
