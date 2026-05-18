import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { AccountMenu } from "../../src/components/AccountMenu";
import { SignInModal } from "../../src/components/SignInModal";

describe("SignInModal", () => {
  it("renders nothing when closed", () => {
    render(
      <SignInModal
        open={false}
        onClose={() => {}}
        onSignIn={async () => ({ ok: true })}
      />,
    );
    expect(screen.queryByTestId("signin-modal")).toBeNull();
  });

  it("renders an email input and explainer text when open", () => {
    render(
      <SignInModal
        open
        onClose={() => {}}
        onSignIn={async () => ({ ok: true })}
      />,
    );
    expect(screen.getByTestId("signin-modal")).toBeInTheDocument();
    expect(screen.getByTestId("signin-email")).toBeInTheDocument();
  });

  it("rejects an invalid email without calling onSignIn", async () => {
    const onSignIn = vi.fn();
    render(<SignInModal open onClose={() => {}} onSignIn={onSignIn} />);
    fireEvent.change(screen.getByTestId("signin-email"), {
      target: { value: "not-an-email" },
    });
    fireEvent.click(screen.getByTestId("signin-submit"));
    expect(onSignIn).not.toHaveBeenCalled();
    expect(screen.getByTestId("signin-error")).toBeInTheDocument();
  });

  it("calls onSignIn with the email and renders the 'check your inbox' state on success", async () => {
    const onSignIn = vi
      .fn()
      .mockResolvedValue({ ok: true, message: "Magic link sent." });
    render(<SignInModal open onClose={() => {}} onSignIn={onSignIn} />);
    fireEvent.change(screen.getByTestId("signin-email"), {
      target: { value: "bee@davisgeometric.com" },
    });
    fireEvent.click(screen.getByTestId("signin-submit"));
    await screen.findByTestId("signin-sent");
    expect(onSignIn).toHaveBeenCalledWith("bee@davisgeometric.com");
  });

  it("surfaces a server error inline without closing", async () => {
    const onSignIn = vi
      .fn()
      .mockResolvedValue({ ok: false, error: "Rate limited" });
    const onClose = vi.fn();
    render(<SignInModal open onClose={onClose} onSignIn={onSignIn} />);
    fireEvent.change(screen.getByTestId("signin-email"), {
      target: { value: "bee@x.com" },
    });
    fireEvent.click(screen.getByTestId("signin-submit"));
    await screen.findByTestId("signin-error");
    expect(screen.getByTestId("signin-error")).toHaveTextContent(/Rate limited/);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes on Escape and on backdrop click", () => {
    const onClose = vi.fn();
    render(
      <SignInModal
        open
        onClose={onClose}
        onSignIn={async () => ({ ok: true })}
      />,
    );
    fireEvent.keyDown(window, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
    onClose.mockClear();
    fireEvent.click(screen.getByTestId("signin-bg"));
    expect(onClose).toHaveBeenCalled();
  });
});

describe("AccountMenu", () => {
  it("renders nothing when closed", () => {
    render(
      <AccountMenu
        open={false}
        email="bee@x.com"
        subscription={null}
        onClose={() => {}}
        onSignOut={async () => {}}
      />,
    );
    expect(screen.queryByTestId("account-menu")).toBeNull();
  });

  it("shows the user's email + sign-out button when open", () => {
    render(
      <AccountMenu
        open
        email="bee@davisgeometric.com"
        subscription={null}
        onClose={() => {}}
        onSignOut={async () => {}}
      />,
    );
    expect(screen.getByTestId("account-menu")).toBeInTheDocument();
    expect(screen.getByTestId("account-email")).toHaveTextContent(
      "bee@davisgeometric.com",
    );
    expect(screen.getByTestId("account-signout")).toBeInTheDocument();
  });

  it("renders the subscription tier when present", () => {
    render(
      <AccountMenu
        open
        email="bee@x.com"
        subscription={{ tier: "founders", status: "active" }}
        onClose={() => {}}
        onSignOut={async () => {}}
      />,
    );
    expect(screen.getByTestId("account-tier")).toHaveTextContent(/founders/i);
  });

  it("calls onSignOut + onClose when sign-out is clicked", async () => {
    const onSignOut = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();
    render(
      <AccountMenu
        open
        email="bee@x.com"
        subscription={null}
        onClose={onClose}
        onSignOut={onSignOut}
      />,
    );
    fireEvent.click(screen.getByTestId("account-signout"));
    // Wait a tick for the async to complete
    await new Promise((r) => setTimeout(r, 0));
    expect(onSignOut).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalled();
  });

  it("closes on backdrop click", () => {
    const onClose = vi.fn();
    render(
      <AccountMenu
        open
        email="bee@x.com"
        subscription={null}
        onClose={onClose}
        onSignOut={async () => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("account-menu-bg"));
    expect(onClose).toHaveBeenCalled();
  });
});
