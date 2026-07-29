import type { FitAddon } from "@xterm/addon-fit";
import type { Terminal } from "@xterm/xterm";
import { type RefObject, useEffect } from "react";
import { sendTerminalClearInput } from "@/lib/terminalControlInput";
import type { PerformanceMode } from "./xterminalTypes";

interface UseTerminalRefreshEffectsParams {
  terminalRef: RefObject<Terminal | null>;
  fitAddonRef: RefObject<FitAddon | null>;
  active: boolean;
  visible: boolean;
  terminalReady: boolean;
  performanceMode: PerformanceMode;
  sessionId: string;
  showGutter: boolean;
  showContentPadding: boolean;
  workspacePaddingSetting?: boolean;
}

export function useTerminalRefreshEffects({
  terminalRef,
  fitAddonRef,
  active,
  visible,
  terminalReady,
  performanceMode,
  sessionId,
  showGutter,
  showContentPadding,
  workspacePaddingSetting,
}: UseTerminalRefreshEffectsParams) {
  useEffect(() => {
    if (terminalReady && fitAddonRef.current && terminalRef.current) {
      requestAnimationFrame(() => {
        fitAddonRef.current?.fit();
        terminalRef.current?.refresh(0, Math.max(0, terminalRef.current.rows - 1));
        if (showGutter && performanceMode === "normal") {
          window.dispatchEvent(
            new CustomEvent("nyaterm:refresh-gutter", {
              detail: { sessionId },
            }),
          );
        }
      });
    }
  }, [fitAddonRef, performanceMode, sessionId, showGutter, terminalReady, terminalRef]);

  useEffect(() => {
    const paddingEnabled = showContentPadding;
    if (!terminalReady || !fitAddonRef.current || !terminalRef.current) return;

    requestAnimationFrame(() => {
      if (paddingEnabled !== (workspacePaddingSetting ?? false)) {
        return;
      }
      fitAddonRef.current?.fit();
      terminalRef.current?.refresh(0, Math.max(0, terminalRef.current.rows - 1));
    });
  }, [fitAddonRef, showContentPadding, terminalReady, terminalRef, workspacePaddingSetting]);

  useEffect(() => {
    if (active && visible && terminalReady && fitAddonRef.current && terminalRef.current) {
      requestAnimationFrame(() => {
        fitAddonRef.current?.fit();
        const terminal = terminalRef.current;
        if (!terminal) return;
        terminal.clearTextureAtlas();
        terminal.refresh(0, Math.max(0, terminal.rows - 1));
        terminal.focus();
      });
    }
  }, [active, fitAddonRef, terminalReady, terminalRef, visible]);

  useEffect(() => {
    const handleRefresh = () => {
      if (!visible || !fitAddonRef.current || !terminalRef.current) return;

      requestAnimationFrame(() => {
        fitAddonRef.current?.fit();
        terminalRef.current?.refresh(0, Math.max(0, terminalRef.current.rows - 1));
        if (active) {
          terminalRef.current?.focus();
        }
      });
    };

    window.addEventListener("nyaterm:refresh-terminals", handleRefresh);
    return () => {
      window.removeEventListener("nyaterm:refresh-terminals", handleRefresh);
    };
  }, [active, fitAddonRef, terminalRef, visible]);

  useEffect(() => {
    const handleClear = () => {
      const terminal = terminalRef.current;
      if (!active || !terminal) return;
      sendTerminalClearInput(terminal, { focus: active });
    };

    window.addEventListener("nyaterm:clear-terminal", handleClear);
    return () => {
      window.removeEventListener("nyaterm:clear-terminal", handleClear);
    };
  }, [active, terminalRef]);
}
