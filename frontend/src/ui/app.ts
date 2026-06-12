import {
  applyMove,
  createGame,
  createSelection,
  isCellSelected,
  playBotMove,
  previewSelection,
} from "../game/engine";
import type { GameState, Rectangle, Selection } from "../game/types";
import { fetchHealth, fetchStaticMove } from "../api/client";

type RenderOptions = {
  selection: Selection;
  message: string;
  online: boolean;
};

export class FruitboxApp {
  private root: HTMLElement;
  private state: GameState;
  private selection: Selection = null;
  private anchor: { x: number; y: number } | null = null;
  private dragging = false;
  private message = "Drag a rectangle whose fruit values add up to the target.";
  private online = false;

  constructor(root: HTMLElement, seed = 42) {
    this.root = root;
    this.state = createGame({ width: 6, height: 6, target: 10, seed });
    this.bindGlobalPointerEnd();
    void this.refreshOnlineStatus();
  }

  mount(): void {
    this.render();
  }

  private bindGlobalPointerEnd(): void {
    window.addEventListener("pointerup", () => {
      if (!this.dragging) {
        return;
      }

      this.dragging = false;
      this.tryCommitSelection();
    });
  }

  private async refreshOnlineStatus(): Promise<void> {
    this.online = await fetchHealth();
    this.render();
  }

  private setMessage(message: string): void {
    this.message = message;
  }

  private resetGame(seed = Date.now()): void {
    this.state = createGame({ width: 6, height: 6, target: 10, seed });
    this.selection = null;
    this.anchor = null;
    this.dragging = false;
    this.setMessage("New board loaded. Select a rectangle that sums to 10.");
    this.render();
  }

  private tryCommitSelection(): void {
    if (!this.selection) {
      this.render();
      return;
    }

    const preview = previewSelection(this.state, this.selection);
    const rectangle = this.selection;
    const { state, result } = applyMove(this.state, rectangle);
    this.selection = null;
    this.anchor = null;
    this.state = state;

    if (result.ok) {
      this.setMessage(`Cleared ${result.cleared} cells. Score is now ${result.score}.`);
    } else if (result.reason === "invalid-sum") {
      this.setMessage(`That rectangle sums to ${preview.sum}, not ${this.state.target}.`);
    } else if (result.reason === "empty-selection") {
      this.setMessage("That selection is already empty.");
    } else if (result.reason === "game-over") {
      this.setMessage("The game is already over.");
    }

    if (this.state.status === "won") {
      this.setMessage(`Board cleared. Final score: ${this.state.score}.`);
    } else if (this.state.status === "stuck") {
      this.setMessage(`No valid moves left. Final score: ${this.state.score}.`);
    }

    this.render();
  }

  private async hintFromServer(): Promise<void> {
    if (this.state.status !== "playing") {
      return;
    }

    try {
      const payload = {
        width: this.state.width,
        height: this.state.height,
        cells: this.state.cells,
        target: this.state.target,
      };

      const response = this.online
        ? await fetchStaticMove(payload)
        : null;

      const move = response?.move
        ? {
            left: response.move.left,
            top: response.move.top,
            right: response.move.right,
            bottom: response.move.bottom,
          }
        : playBotMove(this.state).move;

      if (!move) {
        this.setMessage("No hint available.");
        this.render();
        return;
      }

      this.selection = move;
      this.setMessage(
        this.online
          ? "Server hint highlighted. Release to apply, or adjust the selection."
          : "Local solver hint highlighted. Release to apply, or adjust the selection.",
      );
      this.render();
      this.tryCommitSelection();
    } catch {
      const { move } = playBotMove(this.state);
      if (!move) {
        this.setMessage("Could not fetch a hint.");
        this.render();
        return;
      }

      this.selection = move;
      this.tryCommitSelection();
    }
  }

  private runLocalBotTurn(): void {
    if (this.state.status !== "playing") {
      return;
    }

    const { state, move } = playBotMove(this.state);
    this.state = state;

    if (!move) {
      this.setMessage("Bot could not find a move.");
    } else {
      this.setMessage(`Bot played a move. Score is now ${this.state.score}.`);
    }

    if (this.state.status === "won") {
      this.setMessage(`Bot cleared the board. Final score: ${this.state.score}.`);
    } else if (this.state.status === "stuck") {
      this.setMessage(`Bot got stuck. Final score: ${this.state.score}.`);
    }

    this.render();
  }

  private onCellPointerDown(x: number, y: number): void {
    if (this.state.status !== "playing") {
      return;
    }

    this.dragging = true;
    this.anchor = { x, y };
    this.selection = createSelection({ x, y }, { x, y });
    this.render();
  }

  private onCellPointerEnter(x: number, y: number): void {
    if (!this.dragging || !this.anchor) {
      return;
    }

    this.selection = createSelection(this.anchor, { x, y });
    this.render({ selection: this.selection, message: this.message, online: this.online });
  }

  private render(options?: Partial<RenderOptions>): void {
    const selection = options?.selection ?? this.selection;
    const message = options?.message ?? this.message;
    const online = options?.online ?? this.online;
    const preview = previewSelection(this.state, selection);

    this.root.innerHTML = `
      <main class="shell">
        <header class="hero">
          <div>
            <p class="eyebrow">Fruitbox</p>
            <h1>Sum the orchard</h1>
            <p class="lede">Drag across the grid to select a rectangle. Clear cells when their values add up to the target.</p>
          </div>
          <div class="status-card">
            <div><span>Target</span><strong>${this.state.target}</strong></div>
            <div><span>Score</span><strong>${this.state.score}</strong></div>
            <div><span>Moves</span><strong>${this.state.moves}</strong></div>
            <div><span>Status</span><strong>${this.state.status}</strong></div>
          </div>
        </header>

        <section class="toolbar">
          <button type="button" data-action="new-game">New game</button>
          <button type="button" data-action="hint">Hint</button>
          <button type="button" data-action="bot-turn">Bot turn</button>
          <span class="pill ${online ? "online" : "offline"}">${online ? "API online" : "Offline engine"}</span>
        </section>

        <p class="message">${message}</p>

        <section class="board-wrap">
          <div
            class="board"
            style="grid-template-columns: repeat(${this.state.width}, minmax(0, 1fr));"
            role="grid"
            aria-label="Fruitbox board"
          >
            ${this.renderCells(selection, preview.valid)}
          </div>
          <aside class="preview">
            <h2>Selection</h2>
            <dl>
              <div><dt>Sum</dt><dd>${preview.sum}</dd></div>
              <div><dt>Area</dt><dd>${preview.area}</dd></div>
              <div><dt>Valid</dt><dd>${preview.valid ? "Yes" : "No"}</dd></div>
            </dl>
            <p class="preview-copy">The gameplay engine runs headlessly in TypeScript today and can later be swapped for the Rust/Wasm core.</p>
          </aside>
        </section>
      </main>
    `;

    this.root.querySelector('[data-action="new-game"]')?.addEventListener("click", () => this.resetGame());
    this.root.querySelector('[data-action="hint"]')?.addEventListener("click", () => void this.hintFromServer());
    this.root.querySelector('[data-action="bot-turn"]')?.addEventListener("click", () => this.runLocalBotTurn());

    this.root.querySelectorAll<HTMLElement>("[data-cell]").forEach((cell) => {
      const x = Number(cell.dataset.x);
      const y = Number(cell.dataset.y);
      cell.addEventListener("pointerdown", (event) => {
        event.preventDefault();
        cell.setPointerCapture(event.pointerId);
        this.onCellPointerDown(x, y);
      });
      cell.addEventListener("pointerenter", () => this.onCellPointerEnter(x, y));
    });
  }

  private renderCells(selection: Selection, valid: boolean): string {
    const cells: string[] = [];

    for (let y = 0; y < this.state.height; y += 1) {
      for (let x = 0; x < this.state.width; x += 1) {
        const value = this.state.cells[y * this.state.width + x] ?? 0;
        const selected = isCellSelected(selection, x, y);
        const classes = [
          "cell",
          selected ? "selected" : "",
          selected && valid ? "valid" : "",
          selected && !valid ? "invalid" : "",
          value === 0 ? "cleared" : "",
        ]
          .filter(Boolean)
          .join(" ");

        cells.push(`
          <button
            type="button"
            class="${classes}"
            data-cell
            data-x="${x}"
            data-y="${y}"
            role="gridcell"
            aria-label="Cell ${x + 1}, ${y + 1}, value ${value}"
          >
            <span class="value">${value === 0 ? "" : value}</span>
          </button>
        `);
      }
    }

    return cells.join("");
  }
}

export function mountApp(root: HTMLElement): FruitboxApp {
  const app = new FruitboxApp(root);
  app.mount();
  return app;
}

export type { GameState, Rectangle, Selection };
