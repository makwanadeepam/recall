import { api } from "../../api";
import { errorMessage } from "../../api/tauri";
import type { KnowledgeItemSummary } from "../../db/models";
import { clear, el } from "../components/dom";
import { icon } from "../components/icons";
import { navigate } from "../components/router";
import { button, emptyState, spinner } from "../components/widgets";
import { relativeTime } from "../format";

export async function renderLibrary(container: HTMLElement): Promise<void> {
  let search = "";
  let activeTag: string | null = null;
  let debounce: number | undefined;

  const searchInput = el("input", {
    id: "library-search",
    class: "search-input",
    type: "search",
    placeholder: "Search titles, content, questions, tags…   ( / )",
    on: {
      input: () => {
        search = searchInput.value.trim();
        window.clearTimeout(debounce);
        debounce = window.setTimeout(loadItems, 180);
      },
    },
  });

  const searchBox = el("div", { class: "search-box" }, icon("search", "search-icon"), searchInput);
  const tagFilter = el("div", { class: "tag-filter" });
  const listEl = el("div", { class: "item-list" });

  const header = el(
    "header",
    { class: "page-header" },
    el(
      "div",
      { class: "page-header-text" },
      el("h1", { class: "page-title" }, icon("library", "title-icon"), el("span", { text: "Library" }))
    ),
    el(
      "div",
      { class: "page-actions" },
      button("New item", { variant: "primary", icon: "plus", onClick: () => navigate("knowledge", { id: "new" }) })
    )
  );

  container.replaceChildren(
    el("div", { class: "page library" }, header, el("div", { class: "toolbar" }, searchBox, tagFilter), listEl)
  );

  await loadTags();
  await loadItems();

  async function loadTags(): Promise<void> {
    let tags: string[] = [];
    try {
      tags = await api.listTags();
    } catch {
      /* non-fatal */
    }
    clear(tagFilter);
    if (tags.length === 0) return;
    tagFilter.appendChild(chip("All", activeTag === null, () => setTag(null)));
    for (const t of tags) tagFilter.appendChild(chip(t, activeTag === t, () => setTag(t)));
  }

  function setTag(tag: string | null): void {
    activeTag = tag;
    loadTags();
    loadItems();
  }

  function chip(label: string, active: boolean, onClick: () => void): HTMLElement {
    return el(
      "button",
      { class: "chip chip-button" + (active ? " active" : ""), on: { click: onClick } },
      el("span", { text: label })
    );
  }

  async function loadItems(): Promise<void> {
    clear(listEl);
    listEl.appendChild(spinner());
    let items: KnowledgeItemSummary[];
    try {
      items = await api.listItems(search || null, activeTag);
    } catch (e) {
      clear(listEl);
      listEl.appendChild(emptyState("info", "Couldn't load items", errorMessage(e)));
      return;
    }
    clear(listEl);
    if (items.length === 0) {
      const isFiltered = !!search || !!activeTag;
      listEl.appendChild(
        emptyState(
          "library",
          isFiltered ? "No matches" : "Your library is empty",
          isFiltered
            ? "Try a different search or clear the tag filter."
            : "Create your first knowledge item to start building recall cards.",
          isFiltered
            ? undefined
            : button("New knowledge item", { variant: "primary", icon: "plus", onClick: () => navigate("knowledge", { id: "new" }) })
        )
      );
      return;
    }
    for (const item of items) listEl.appendChild(itemCard(item));
  }
}

function itemCard(item: KnowledgeItemSummary): HTMLElement {
  const badges = el("div", { class: "list-badges" });
  badges.appendChild(el("span", { class: "count-badge", text: `${item.card_count} card${item.card_count === 1 ? "" : "s"}` }));
  if (item.due_count > 0) {
    badges.appendChild(el("span", { class: "count-badge due", text: `${item.due_count} due` }));
  }

  const tags = el("div", { class: "list-tags" });
  for (const t of item.tags) tags.appendChild(el("span", { class: "chip tiny" }, icon("tag", "chip-icon"), el("span", { text: t })));

  return el(
    "button",
    { class: "list-card", on: { click: () => navigate("knowledge", { id: item.id }) } },
    el("div", { class: "list-card-head" }, el("span", { class: "list-card-title", text: item.title }), badges),
    item.excerpt.trim() ? el("div", { class: "list-card-excerpt muted", text: item.excerpt }) : null,
    el(
      "div",
      { class: "list-card-foot" },
      tags,
      el("time", { class: "muted", text: `updated ${relativeTime(item.updated_at)}` })
    )
  );
}
