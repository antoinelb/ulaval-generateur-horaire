You are an expert [Dioxus 0.7](https://dioxuslabs.com/learn/0.7) assistant. Dioxus 0.7 changed every API: `cx`, `Scope`, `use_state`, and `use_ref` are gone. Use `use_signal`, `#[component]`, `rsx!`, `Routable`, `use_resource` only, as documented here.

This project is **client-side rendered, fully static** (GitHub Pages). Never introduce `fullstack`, server functions (`#[post]`/`#[get]`), hydration, or `use_server_future` — the solver runs in the browser and in a Web Worker; there is no server.

# Antipatterns — hard rules from production bugs

Every rule below exists because the bug shipped (see the ADRs). When a new Dioxus-misuse bug is fixed, add its antipattern here in the same change, with a minimal bad/good pair.

## AP-1 — Never hold a `.read()` guard when writing (`AlreadyBorrowed`)

A temporary `signal.read()` living in an `if` condition or a `match` scrutinee survives to the end of the whole statement; any `signal.write()` inside the branch/arm **panics at runtime** (`AlreadyBorrowed`). This shipped twice in one day (panel, grid).

```rust
// BAD — read guard is still alive inside the branch
if plan.read().sessions.is_empty() {
	plan.write().sessions.push(default_session()); // panic: AlreadyBorrowed
}

// GOOD — materialize the read into a `let` (copy/clone out), then write
let is_empty = plan.read().sessions.is_empty();
if is_empty {
	plan.write().sessions.push(default_session());
}
```

Rule: **never call `.read()` inline inside the condition of `if`/`while`, the scrutinee of `match`, or any expression that also writes**. Extract the value first (`let x = signal.read().field.clone();` or `signal()` for a full clone), drop the guard, then write. Same for nested writes: one live guard per signal, ever.

## AP-2 — Never hold a guard across `.await` or across another signal operation

Guards (`.read()` and `.write()`) are synchronous borrows. Holding one across an `.await` point, a `spawn`, or any call that may itself read/write the same signal is the same panic deferred.

```rust
// BAD
let data = resource.read();
save(&data).await; // guard alive across await

// GOOD
let data = resource.read().clone();
save(&data).await;
```

## AP-3 — Never write a signal from a reactive closure that reads it

A `use_memo`/`use_effect` that reads a signal re-runs when it changes; writing that same signal inside it is an infinite re-run (or a panic). Memos **derive**, effects **observe**; only event handlers and `edit_plan` mutate.

```rust
// BAD — memo writes what it reads
let total = use_memo(move || { count.set(count() + 1); count() });

// GOOD — memo is a pure function of its inputs
let total = use_memo(move || plan.read().sessions.iter().map(credits).sum::<u32>());
```

## AP-4 — Hooks are unconditional, top-of-component, fixed order

`use_signal`, `use_memo`, `use_resource`, `use_context*`, `use_effect` must be called unconditionally at the top of the component body — never inside `if`, loops, event handlers, or after an early return. Conditional hooks corrupt hook state across renders.

## AP-5 — Zero logic in rsx

The coverage regex excludes `components/`: **any branch written in rsx escapes testing entirely**. Every `if`/`for` in rsx must be fed by an already-computed value from a pure module (`state`, `present`, `data`, `solve`, `persist`) via a memo. If you are about to write a comparison, arithmetic, filter, or format inside `rsx! { }`, stop and move it to the pure module, with its test.

```rust
// BAD — decision made in the view, untested
rsx! { if plan.read().credits() > program.read().max { Warning {} } }

// GOOD — pure module decides, view renders
let over_cap = use_memo(move || present::over_credit_cap(&plan.read(), &program.read()));
rsx! { if over_cap() { Warning {} } }
```

## AP-6 — Every mutation goes through `edit_plan` → `state::apply`

Components never mutate `Plan`/`View`/`History` directly. Two doors exist: `edit_plan` for editing the current document — labeled and undoable (ACT-2 — no confirmation dialogs, always reversible) — and `swap_document` for replacing it (« changer », « Choisir »), which shelves the old document, installs the next one whole and resets `History`/`View` (ADR `2026-08-historique-par-document-vide-a-la-bascule`). A `.write()` on plan state anywhere else is a review-blocking defect.

## AP-7 — Respect the wasm32 boundary

`components/` and `browser.rs` exist only under `cfg(target_arch = "wasm32")`. Never import `web-sys`, `gloo`, `wasm-bindgen`, or write rsx in the pure modules — they must compile and reach 100% coverage natively under `make test`. Inversely, never put testable logic in the wasm32-only files (AP-5).

## AP-8 — Keyed loops use stable domain identity, never indices

```rust
// BAD
for (i, course) in courses.iter().enumerate() { CoursePill { key: "{i}", .. } }

// GOOD — the sigle is the identity
for course in courses.iter() { CoursePill { key: "{course.code}", .. } }
```

Index keys break drag-and-drop and animation identity the moment the list reorders.

```rust
// BAD — no key at all: positional diff reuses a scope at its position,
// so a handler that captured values by move fires with the *previous*
// occupant's data once the list reorders
for row in rows.iter().cloned() { RowView { row } }

// GOOD
for row in rows.iter().cloned() { RowView { key: "{row.code}", row } }
```

A missing key is worse than an index key: the reused component isn't re-rendered, so every closure inside it (onclick, oninput) keeps the stale captures of the render that created it.

## AP-9 — Props are owned, `PartialEq + Clone`; signals are `Copy` — pass them

Props take `String`/`Vec<T>`, never `&str`/`&[T]`. For reactive props use `ReadOnlySignal<T>`; for callbacks use `EventHandler<T>`. `Signal<T>` is `Copy`: pass it by value into children and closures — never a reference, never `Rc` wrapping.

## AP-10 — Reading in the view: call the signal, don't store guards

In rsx and handlers, prefer `signal()` (clones the value) or one immediate `.read()` used and dropped in the same expression. Never bind a guard to a variable that outlives the line (`let r = signal.read();` at component top is a latent AP-1).

# API reference (0.7)

## Launch

```rust
use dioxus::prelude::*;

fn main() {
	dioxus::launch(App);
}

#[component]
fn App() -> Element {
	rsx! { "Hello, Dioxus!" }
}
```

## rsx

```rust
rsx! {
	div {
		class: "container",              // attribute
		width: if cond { "100%" },       // conditional attribute
		"text"
	}
	for item in items.iter() {           // prefer for over iterators
		Row { key: "{item.id}" }         // AP-8: stable key
	}
	if cond {
		div { "true branch" }            // AP-5: cond comes from a memo
	}
	{children}                           // expressions in braces
}
```

## Components and props

- Functions annotated `#[component]`, name capitalized.
- Re-render iff props change (`PartialEq`) or a read signal/memo changes.
- Owned props; `ReadOnlySignal<T>` for reactive props; `EventHandler<T>` for callbacks (AP-9).

```rust
#[component]
fn Input(mut value: Signal<String>, onsubmit: EventHandler<String>) -> Element {
	rsx! {
		input {
			value,
			oninput: move |e| *value.write() = e.value(),
			onkeydown: move |e| {
				if e.key() == Key::Enter {
					let v = value();          // AP-1: materialize before write
					value.write().clear();
					onsubmit.call(v);
				}
			},
		}
	}
}
```

## State

```rust
let mut count = use_signal(|| 0);
let doubled = use_memo(move || count() * 2);   // pure derivation (AP-3)
count.with_mut(|c| *c += 1);                   // scoped mutation, guard-safe
```

- `signal()` clones the value; `.read()`/`.write()` return guards (AP-1/AP-2/AP-10).
- `use_effect` for post-render side effects only (focus, scroll, browser IO) — never for deriving state.

## Context

```rust
// provider (parent)
use_context_provider(|| Signal::new(Plan::default()));
// consumer (any descendant)
let plan = use_context::<Signal<Plan>>();
```

## Async

```rust
let dog = use_resource(move || async move { fetch_dog().await }); // re-runs when read signals change
match dog() {
	Some(info) => rsx! { Dog { info } },
	None => rsx! { "Chargement…" },
}
```

In event handlers, use `spawn(async move { … })`; clone out of guards before the first `.await` (AP-2).

## Routing

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
	#[layout(NavBar)]
		#[route("/")]
		Home {},
		#[route("/plan/:code")]
		Plan { code: String },
}

#[component]
fn NavBar() -> Element {
	rsx! { a { href: "/", "Accueil" } Outlet::<Route> {} }
}

#[component]
fn App() -> Element {
	rsx! { Router::<Route> {} }
}
```

## Assets

```rust
rsx! {
	document::Stylesheet { href: asset!("/assets/main.css") }
	img { src: asset!("/assets/favicon.ico"), alt: "…" }
}
```
