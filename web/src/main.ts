import { registerSW } from "virtual:pwa-register";
import { fallbackModes, findBestMove, loadModes, type GameMode } from "./api";
import "./styles.css";
import {
  applyRectangle,
  createBoard,
  findStaticMoves,
  initFruitboxWasm,
  isInside,
  rectangleFromCells,
  scoreRectangle,
  sumRectangle,
  type Rectangle
} from "./solver";

const root = document.querySelector<HTMLDivElement>("#app");

if (!root) {
  throw new Error("Missing #app root element");
}

const appRoot = root;

await initFruitboxWasm();

let board = createBoard();
let score = 0;
let moveCount = 0;
let modes: GameMode[] = fallbackModes();
let statusMessage = "Select a rectangle whose fruit total is 10.";
let selectionStart: number | null = null;
let selectionEnd: number | null = null;
let hint: Rectangle | null = null;

registerSW({
  onOfflineReady() {
    statusMessage = "Fruitbox is cached and ready for offline singleplayer.";
    render();
  },
  onNeedRefresh() {
    statusMessage = "A new version is available. Refresh when you are ready.";
    render();
  }
});

window.addEventListener("online", () => {
  statusMessage = "Back online. Solver hints can use the backend again.";
  render();
});

window.addEventListener("offline", () => {
  statusMessage = "Offline mode. Singleplayer and the local static solver still work.";
  render();
});

window.addEventListener("pointermove", (event) => {
  if (selectionStart === null) {
    return;
  }

  const index = cellIndexFromPoint(event.clientX, event.clientY);
  if (index === null || index === selectionEnd) {
    return;
  }

  selectionEnd = index;
  render();
});

window.addEventListener("pointerup", (event) => {
  if (selectionStart === null) {
    return;
  }

  const index = cellIndexFromPoint(event.clientX, event.clientY);
  if (index === null) {
    clearSelection();
    render();
    return;
  }

  selectionEnd = index;
  submitSelection();
});

loadModes()
  .then((loadedModes) => {
    modes = loadedModes;
    render();
  })
  .catch(() => {
    statusMessage = "Using locally cached mode metadata until the API is reachable.";
    render();
  });

render();

function render() {
  const activeSelection = getActiveSelection();
  const selectionSum = activeSelection ? sumRectangle(board, activeSelection) : 0;
  const selectionScore = activeSelection ? scoreRectangle(board, activeSelection).score : 0;
  const remainingMoves = findStaticMoves(board).length;

  appRoot.innerHTML = `
    <main class="shell">
      <section class="hero">
        <div>
          <p class="eyebrow">Fruitbox PWA</p>
          <h1>Clear fruit by boxing sums of ${board.target}.</h1>
          <p class="lede">
            Singleplayer is playable now, with an offline static solver fallback.
            Multiplayer and heavier RL/NN bots are reserved as online-first modes.
          </p>
          <div class="actions">
            <button class="primary" data-action="new-game">New board</button>
            <button data-action="hint">Static solver hint</button>
          </div>
        </div>
        <div class="score-card">
          <span>Score</span>
          <strong>${score}</strong>
          <small>${moveCount} moves &middot; ${remainingMoves} possible</small>
        </div>
      </section>

      <section class="game-layout">
        <div class="panel board-panel">
          <div class="board-heading">
            <div>
              <h2>Singleplayer board</h2>
              <p>${statusMessage}</p>
            </div>
            <span class="network ${navigator.onLine ? "online" : "offline"}">
              ${navigator.onLine ? "Online" : "Offline"}
            </span>
          </div>

          <div
            class="board"
            style="--columns: ${board.width}"
            aria-label="Fruitbox game board"
          >
            ${board.cells.map((cell, index) => renderCell(cell, index, activeSelection)).join("")}
          </div>

          <div class="selection-readout ${selectionSum === board.target ? "valid" : ""}">
            <span>Selection sum: ${selectionSum}</span>
            <span>Fruits: ${selectionScore}</span>
          </div>
        </div>

        <aside class="panel">
          <h2>Modes</h2>
          <div class="mode-list">
            ${modes.map(renderMode).join("")}
          </div>
        </aside>
      </section>
    </main>
  `;

  attachEvents();
}

function renderCell(
  cell: number,
  index: number,
  activeSelection: Omit<Rectangle, "score"> | null
): string {
  const classes = [
    "cell",
    cell === 0 ? "empty" : "",
    isInside(activeSelection, index, board.width) ? "selected" : "",
    isInside(hint, index, board.width) ? "hint" : ""
  ]
    .filter(Boolean)
    .join(" ");

  return `
    <button class="${classes}" data-cell-index="${index}" aria-label="Cell ${index + 1}">
      ${cell === 0 ? "" : cell}
    </button>
  `;
}

function renderMode(mode: GameMode): string {
  return `
    <article class="mode-card">
      <div>
        <h3>${mode.label}</h3>
        <p>${mode.description}</p>
      </div>
      <span>${mode.offline_capable ? "Offline-capable" : "Online"}</span>
    </article>
  `;
}

function attachEvents() {
  appRoot.querySelector<HTMLButtonElement>('[data-action="new-game"]')?.addEventListener("click", () => {
    board = createBoard(board.width, board.height, board.target);
    score = 0;
    moveCount = 0;
    hint = null;
    statusMessage = "New board ready. Select a rectangle that sums to 10.";
    render();
  });

  appRoot.querySelector<HTMLButtonElement>('[data-action="hint"]')?.addEventListener("click", () => {
    void showHint();
  });

  appRoot.querySelectorAll<HTMLButtonElement>("[data-cell-index]").forEach((cell) => {
    cell.addEventListener("pointerdown", (event) => {
      event.preventDefault();

      const index = cellIndexFromEvent(event);
      if (index === null) {
        return;
      }

      selectionStart = index;
      selectionEnd = index;
      hint = null;
      render();
    });
  });
}

async function showHint() {
  statusMessage = "Checking the static solver...";
  render();

  const result = await findBestMove(board);
  hint = result.move;

  if (result.move) {
    statusMessage = `${result.source === "api" ? "Backend" : "Local"} solver highlighted a ${result.move.score}-fruit move.`;
  } else {
    statusMessage = "No moves remain. Start a new board or keep experimenting.";
  }

  render();
}

function submitSelection() {
  const activeSelection = getActiveSelection();

  if (!activeSelection) {
    return;
  }

  const rectangle = scoreRectangle(board, activeSelection);
  const sum = sumRectangle(board, rectangle);

  if (sum === board.target && rectangle.score > 0) {
    board = applyRectangle(board, rectangle);
    score += rectangle.score;
    moveCount += 1;
    hint = null;
    statusMessage = `Nice: +${rectangle.score} fruits.`;
  } else {
    statusMessage = `That box totals ${sum}; aim for ${board.target}.`;
  }

  clearSelection();
  render();
}

function getActiveSelection(): Omit<Rectangle, "score"> | null {
  if (selectionStart === null || selectionEnd === null) {
    return null;
  }

  return rectangleFromCells(selectionStart, selectionEnd, board.width);
}

function clearSelection() {
  selectionStart = null;
  selectionEnd = null;
}

function cellIndexFromEvent(event: Event): number | null {
  const target = event.currentTarget;

  if (!(target instanceof HTMLElement)) {
    return null;
  }

  const value = target.dataset.cellIndex;
  return value ? Number.parseInt(value, 10) : null;
}

function cellIndexFromPoint(clientX: number, clientY: number): number | null {
  const target = document.elementFromPoint(clientX, clientY)?.closest("[data-cell-index]");

  if (!(target instanceof HTMLElement)) {
    return null;
  }

  const value = target.dataset.cellIndex;
  return value ? Number.parseInt(value, 10) : null;
}
