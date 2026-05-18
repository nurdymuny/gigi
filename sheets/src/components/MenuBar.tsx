import { useEffect, useRef, useState } from "react";
import "./MenuBar.css";

/**
 * MenuBar — Excel/Airtable-style top menu strip.
 *
 * One File/Edit/View/Insert/Format/Data/Geometry/Tools/Help dropdown.
 * Click a top-level menu to open it; mouse-over another to switch.
 * Items dispatch actions to the App via the `actions` map; anything not
 * handled there falls back to `onAction` with a sentinel id so the App
 * can toast "coming soon" for stubs.
 */

export interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  separator?: boolean;
  section?: string;
  /** When `check()` returns true, a ✓ shows on the left. */
  check?: () => boolean;
  submenu?: MenuItem[];
}

export interface Menu {
  name: string;
  items: MenuItem[];
}

export interface MenuBarProps {
  menus: Menu[];
  /** Dispatch — receives the item id. Return value not used. */
  onAction: (id: string) => void;
}

export function MenuBar({ menus, onAction }: MenuBarProps) {
  const [openName, setOpenName] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!openName) return;
    function onDocClick(e: MouseEvent) {
      if (!ref.current?.contains(e.target as Node)) setOpenName(null);
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpenName(null);
    }
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [openName]);

  const onItemClick = (item: MenuItem) => {
    if (item.submenu) return; // submenu open handled inline
    setOpenName(null);
    onAction(item.id);
  };

  return (
    <nav className="menubar" data-testid="menubar" ref={ref} role="menubar">
      {menus.map((m) => {
        const isOpen = openName === m.name;
        return (
          <div className="menu-wrap" key={m.name}>
            <button
              type="button"
              className={`menu-btn ${isOpen ? "menu-btn-open" : ""}`}
              data-testid={`menu-${m.name.toLowerCase()}`}
              aria-haspopup="menu"
              aria-expanded={isOpen}
              onClick={(e) => {
                e.stopPropagation();
                setOpenName(isOpen ? null : m.name);
              }}
              onMouseEnter={() => {
                if (openName && openName !== m.name) setOpenName(m.name);
              }}
            >
              {m.name}
            </button>
            {isOpen ? (
              <MenuDropdown
                items={m.items}
                onItemClick={onItemClick}
                onAction={(id) => {
                  setOpenName(null);
                  onAction(id);
                }}
              />
            ) : null}
          </div>
        );
      })}
    </nav>
  );
}

function MenuDropdown({
  items,
  onItemClick,
  onAction,
}: {
  items: MenuItem[];
  onItemClick: (item: MenuItem) => void;
  onAction: (id: string) => void;
}) {
  const [openSub, setOpenSub] = useState<string | null>(null);
  return (
    <div className="menu-dropdown" role="menu">
      {items.map((it, i) => {
        if (it.section) {
          return (
            <div key={`section-${i}`} className="menu-section">
              {it.section}
            </div>
          );
        }
        if (it.separator) {
          return <div key={`sep-${i}`} className="menu-sep" />;
        }
        const isChecked = it.check?.() ?? false;
        const hasSub = Boolean(it.submenu && it.submenu.length > 0);
        const isSubOpen = openSub === it.id;
        return (
          <div className="menu-item-wrap" key={it.id}>
            <button
              type="button"
              role="menuitem"
              className={`menu-item ${hasSub ? "menu-item-has-sub" : ""}`}
              data-testid={`menu-item-${it.id}`}
              onMouseEnter={() => setOpenSub(hasSub ? it.id : null)}
              onClick={(e) => {
                e.stopPropagation();
                if (hasSub) {
                  setOpenSub(isSubOpen ? null : it.id);
                } else {
                  onItemClick(it);
                }
              }}
            >
              <span className="menu-check">{isChecked ? "✓" : ""}</span>
              <span className="menu-label">{it.label}</span>
              <span className="menu-shortcut">
                {hasSub ? "▸" : (it.shortcut ?? "")}
              </span>
            </button>
            {hasSub && isSubOpen ? (
              <div className="menu-submenu" role="menu">
                {it.submenu!.map((sub) => (
                  <button
                    type="button"
                    key={sub.id}
                    role="menuitem"
                    className="menu-item"
                    data-testid={`menu-item-${sub.id}`}
                    onClick={(e) => {
                      e.stopPropagation();
                      onAction(sub.id);
                    }}
                  >
                    <span className="menu-check"></span>
                    <span className="menu-label">{sub.label}</span>
                    <span className="menu-shortcut">{sub.shortcut ?? ""}</span>
                  </button>
                ))}
              </div>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}
