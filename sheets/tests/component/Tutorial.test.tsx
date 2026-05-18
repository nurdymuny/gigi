import { describe, expect, it, vi } from "vitest";
import {
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { Tutorial, type TutorialStep } from "../../src/components/Tutorial";

function makeSteps(): TutorialStep[] {
  return [
    {
      title: "Welcome",
      body: <p>Hello world.</p>,
      target: null,
    },
    {
      title: "Find the heading",
      body: <p>This step targets a real DOM element.</p>,
      target: "tour-target-heading",
    },
    {
      title: "Done",
      body: <p>That's it.</p>,
      target: null,
    },
  ];
}

describe("Tutorial", () => {
  it("does not render when open is false", () => {
    render(
      <Tutorial open={false} steps={makeSteps()} onClose={() => undefined} />,
    );
    expect(screen.queryByTestId("tutorial-root")).not.toBeInTheDocument();
  });

  it("renders the first step when open", async () => {
    render(
      <div>
        <h1 data-testid="tour-target-heading">Targetable</h1>
        <Tutorial open steps={makeSteps()} onClose={() => undefined} />
      </div>,
    );
    expect(screen.getByTestId("tutorial-root")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-card")).toHaveTextContent("Welcome"),
    );
    expect(screen.getByTestId("tutorial-card")).toHaveTextContent(
      "Step 1 of 3",
    );
  });

  it("advances to the next step on Next click", async () => {
    render(
      <div>
        <h1 data-testid="tour-target-heading">Targetable</h1>
        <Tutorial open steps={makeSteps()} onClose={() => undefined} />
      </div>,
    );
    // Wait until the step's settle delay elapses and the Next button
    // is no longer in its "Loading…" busy state.
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-next")).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByTestId("tutorial-next"));
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-card")).toHaveTextContent(
        "Find the heading",
      ),
    );
    expect(screen.getByTestId("tutorial-card")).toHaveTextContent("Step 2 of 3");
  });

  it("Back button is disabled on the first step", async () => {
    render(
      <Tutorial open steps={makeSteps()} onClose={() => undefined} />,
    );
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-back")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("tutorial-back")).toBeDisabled();
  });

  it("Skip calls onClose immediately", async () => {
    const onClose = vi.fn();
    render(<Tutorial open steps={makeSteps()} onClose={onClose} />);
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-skip")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByTestId("tutorial-skip"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("Finish on the last step calls onClose", async () => {
    const onClose = vi.fn();
    const steps: TutorialStep[] = [{ title: "Only", body: <p>x</p>, target: null }];
    render(<Tutorial open steps={steps} onClose={onClose} />);
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-next")).toHaveTextContent(/finish/i),
    );
    fireEvent.click(screen.getByTestId("tutorial-next"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("runs a step's action when that step becomes active", async () => {
    const action = vi.fn().mockResolvedValue(undefined);
    const steps: TutorialStep[] = [
      { title: "A", body: <p>a</p>, target: null, action },
    ];
    render(<Tutorial open steps={steps} onClose={() => undefined} />);
    await waitFor(() => expect(action).toHaveBeenCalledTimes(1));
  });

  it("does not crash when a step's target does not exist", async () => {
    const steps: TutorialStep[] = [
      { title: "Missing", body: <p>x</p>, target: "does-not-exist-at-all" },
    ];
    render(<Tutorial open steps={steps} onClose={() => undefined} />);
    await waitFor(() =>
      expect(screen.getByTestId("tutorial-card")).toHaveTextContent("Missing"),
    );
    // Spotlight ring is absent because no element matched.
    expect(screen.queryByText(/Spotlight/)).not.toBeInTheDocument();
  });
});
