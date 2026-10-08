import { api } from "../../api";
import { errorMessage } from "../../api/tauri";
import type { DashboardStats } from "../../db/models";
import { el } from "../components/dom";
import { icon } from "../components/icons";
import { navigate } from "../components/router";
import { button, emptyState, ratingBadge, spinner, statTile } from "../components/widgets";
import { relativeTime } from "../format";

export async function renderDashboard(container: HTMLElement): Promise<void> {
  container.appendChild(spinner());
  let stats: DashboardStats;
  try {
    stats = await api.dashboard();
  } catch (e) {
    container.replaceChildren(emptyState("info", "Couldn't load the dashboard", errorMessage(e)));
    return;
  }

  const startBtn = button("Start Recall", {
    variant: "primary",
    icon: "play",
    class: "btn-large",
    onClick: () => navigate("recall"),
  });

  const actionable = stats.due_now + stats.new_cards;
  const todayHint =
    actionable > 0
      ? `${actionable} card${actionable === 1 ? "" : "s"} ready to review`
      : "You're all caught up for now";

  const today = el(
    "section",
    { class: "today-card" },
    el(
      "div",
      { class: "today-main" },
      el("div", { class: "today-eyebrow", text: "Today" }),
      el("div", { class: "today-hint", text: todayHint }),
      el(
        "div",
        { class: "today-metrics" },
        metric("Due", stats.due_now),
        metric("New", stats.new_cards),
        metric("Reviewed", stats.reviewed_today)
      )
    ),
    el("div", { class: "today-cta" }, startBtn)
  );

  const grid = el(
    "div",
    { class: "stat-grid" },
    statTile("flame", stats.streak === 1 ? "day streak" : "day streak", stats.streak, "orange"),
    statTile("layers", "total cards", stats.total_cards, "indigo"),
    statTile("award", "mastered", stats.mastered, "green"),
    statTile("recall", "due today", stats.due_today, "blue")
  );

  const activity = el("section", { class: "panel" }, el("h2", { class: "panel-title", text: "Recent activity" }));
  if (stats.recent_activity.length === 0) {
    activity.appendChild(el("p", { class: "muted pad", text: "No reviews yet. Start a session to see your history here." }));
  } else {
    const list = el("ul", { class: "activity-list" });
    for (const a of stats.recent_activity) {
      list.appendChild(
        el(
          "li",
          { class: "activity-item" },
          ratingBadge(a.rating),
          el(
            "div",
            { class: "activity-text" },
            el("div", { class: "activity-question", text: a.question }),
            el("div", { class: "activity-source muted", text: a.knowledge_title })
          ),
          el("time", { class: "activity-time muted", text: relativeTime(a.reviewed_at) })
        )
      );
    }
    activity.appendChild(list);
  }

  const quicklinks = el(
    "div",
    { class: "quicklinks" },
    button("New knowledge item", { icon: "plus", onClick: () => navigate("knowledge", { id: "new" }) }),
    button("Browse library", { icon: "library", variant: "ghost", onClick: () => navigate("library") })
  );

  container.replaceChildren(
    el("div", { class: "page dashboard" }, greeting(), today, grid, quicklinks, activity)
  );
}

function metric(label: string, value: number): HTMLElement {
  return el(
    "div",
    { class: "metric" },
    el("div", { class: "metric-value", text: String(value) }),
    el("div", { class: "metric-label", text: label })
  );
}

function greeting(): HTMLElement {
  const hour = new Date().getHours();
  const word = hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
  return el(
    "header",
    { class: "page-header" },
    el(
      "div",
      { class: "page-header-text" },
      el("h1", { class: "page-title" }, icon("home", "title-icon"), el("span", { text: word })),
      el("p", { class: "page-subtitle", text: "Your active recall at a glance." })
    )
  );
}
