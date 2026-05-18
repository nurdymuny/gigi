import { useEffect, useState } from "react";
import {
  deleteView,
  encodeView,
  listViews,
  saveView,
  type NamedView,
  type ViewSpec,
} from "../lib/view";
import "./ViewsDrawer.css";

export interface ViewsDrawerProps {
  open: boolean;
  bundle: string;
  currentSpec: ViewSpec;
  onClose: () => void;
  onApply: (spec: ViewSpec) => void;
  /** Called after the user clicks "Copy link" — receives the share URL. */
  onShare?: (url: string) => void;
}

export function ViewsDrawer({
  open,
  bundle,
  currentSpec,
  onClose,
  onApply,
  onShare,
}: ViewsDrawerProps) {
  const [views, setViews] = useState<NamedView[]>([]);
  const [newName, setNewName] = useState("");

  useEffect(() => {
    if (open) setViews(listViews(bundle));
  }, [open, bundle]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const handleSave = () => {
    if (!newName.trim()) return;
    saveView({ name: newName.trim(), bundle, spec: currentSpec });
    setViews(listViews(bundle));
    setNewName("");
  };

  const handleDelete = (id: string) => {
    deleteView(id);
    setViews(listViews(bundle));
  };

  const buildShareUrl = (spec: ViewSpec) => {
    if (typeof window === "undefined") return "";
    const url = new URL(window.location.href);
    const params = new URLSearchParams();
    params.set("view", encodeView(spec));
    url.search = params.toString();
    return url.toString();
  };

  return (
    <>
      <div
        className="views-drawer-bg"
        data-testid="views-drawer-bg"
        onClick={onClose}
      />
      <aside
        className="views-drawer"
        data-testid="views-drawer"
        role="dialog"
        aria-label="Saved views"
      >
        <header className="views-drawer-head">
          <h3>Saved views</h3>
          <button
            type="button"
            className="views-drawer-close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </header>

        <div className="views-drawer-section">
          <label className="views-drawer-savelabel">
            Save current state as…
          </label>
          <form
            className="views-drawer-save"
            onSubmit={(e) => {
              e.preventDefault();
              handleSave();
            }}
          >
            <input
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. North-3 anomalies"
              data-testid="views-drawer-name"
            />
            <button
              type="submit"
              className="views-drawer-btn views-drawer-btn-primary"
              disabled={!newName.trim()}
              data-testid="views-drawer-save"
            >
              Save view
            </button>
          </form>
          <button
            type="button"
            className="views-drawer-share"
            data-testid="views-drawer-copy-link"
            onClick={() => {
              const url = buildShareUrl(currentSpec);
              navigator.clipboard.writeText(url).then(() => onShare?.(url));
            }}
            title="Copy a URL that re-applies the current view state"
          >
            Copy share link
          </button>
        </div>

        <div className="views-drawer-list-wrap">
          {views.length === 0 ? (
            <p className="views-drawer-empty" data-testid="views-drawer-empty">
              No saved views for <code>{bundle}</code> yet.
            </p>
          ) : (
            <ul className="views-drawer-list" data-testid="views-drawer-list">
              {views.map((v) => (
                <li
                  key={v.id}
                  className="views-drawer-item"
                  data-testid={`views-drawer-item-${v.id}`}
                >
                  <button
                    type="button"
                    className="views-drawer-apply"
                    onClick={() => onApply(v.spec)}
                  >
                    <span className="views-drawer-name">{v.name}</span>
                    <span className="views-drawer-sub">
                      {v.spec.activeView ?? "grid"} · cover{" "}
                      {v.spec.coverField ?? "—"}
                    </span>
                  </button>
                  <button
                    type="button"
                    className="views-drawer-trash"
                    onClick={() => handleDelete(v.id)}
                    aria-label={`Delete view ${v.name}`}
                    data-testid={`views-drawer-delete-${v.id}`}
                  >
                    Delete
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </aside>
    </>
  );
}

