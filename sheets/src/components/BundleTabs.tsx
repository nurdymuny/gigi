import "./BundleTabs.css";

export interface BundleTabsProps {
  /** Names of bundles currently open, in display order. */
  tabs: string[];
  /** Currently-active bundle (the one in the URL). */
  active: string | null;
  /** Switch the active bundle. */
  onSelect: (name: string) => void;
  /** Close a tab. If it was active, the parent decides where to fall back. */
  onClose: (name: string) => void;
  /** Open the picker to add a new bundle. */
  onAdd: () => void;
}

/**
 * Browser-style bundle tabs across the top of the main area. One tab
 * per open bundle; click to switch active; × to close. Plus a + button
 * that opens the picker for adding more.
 */
export function BundleTabs({
  tabs,
  active,
  onSelect,
  onClose,
  onAdd,
}: BundleTabsProps) {
  if (tabs.length === 0) return null;
  return (
    <div className="bundle-tabs" role="tablist" data-testid="bundle-tabs">
      {tabs.map((name) => {
        const isActive = name === active;
        return (
          <div
            key={name}
            className={`bundle-tab ${isActive ? "bundle-tab-active" : ""}`}
            role="tab"
            aria-selected={isActive}
            data-testid={`bundle-tab-${name}`}
            data-active={isActive ? "true" : "false"}
          >
            <button
              type="button"
              className="bundle-tab-label"
              onClick={() => onSelect(name)}
              data-testid={`bundle-tab-select-${name}`}
              title={name}
            >
              <span className="bundle-tab-dot" aria-hidden="true" />
              {name}
            </button>
            {tabs.length > 1 ? (
              <button
                type="button"
                className="bundle-tab-close"
                onClick={(e) => {
                  e.stopPropagation();
                  onClose(name);
                }}
                aria-label={`Close ${name}`}
                data-testid={`bundle-tab-close-${name}`}
                title={`Close ${name}`}
              >
                ×
              </button>
            ) : null}
          </div>
        );
      })}
      <button
        type="button"
        className="bundle-tab-add"
        onClick={onAdd}
        aria-label="Open another bundle"
        title="Open another bundle"
        data-testid="bundle-tab-add"
      >
        +
      </button>
    </div>
  );
}
