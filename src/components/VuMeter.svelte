<script lang="ts">
  import { onMount } from 'svelte';
  import { onVuMeter, type VuMeterData } from '../lib/ipc';

  /* ── State & Props ───────────────────────── */
  let { vertical = false }: { vertical?: boolean } = $props();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;
  let animFrameId: number;
  let containerEl: HTMLDivElement;

  // Current smoothed values (for animation)
  let currentLeft = -60;
  let currentRight = -60;
  let peakLeft = -60;
  let peakRight = -60;

  // Target values received from IPC
  let targetLeft = -60;
  let targetRight = -60;
  let targetPeakLeft = -60;
  let targetPeakRight = -60;

  // Peak hold with decay
  let peakHoldLeft = -60;
  let peakHoldRight = -60;
  let peakHoldLeftTimer = 0;
  let peakHoldRightTimer = 0;

  const PEAK_HOLD_MS = 1200;
  const PEAK_DECAY_RATE = 0.15; // dB per frame
  const DB_MIN = -60;
  const DB_MAX = 0;

  /* ── dBFS to pixel fraction ─────────────── */
  function dbToFraction(db: number): number {
    const clamped = Math.max(DB_MIN, Math.min(DB_MAX, db));
    return (clamped - DB_MIN) / (DB_MAX - DB_MIN);
  }

  /* ── Grid line positions (dBFS) ─────────── */
    const gridLines = [-48, -36, -24, -12, -6, -3, 0];
 
  /* ── Dynamic Color Cache ─────────────────── */
  let vuColors = {
    bg: '#0a0a12',
    divider: '#0a0a12',
    text: 'rgba(148, 163, 184, 0.55)',
    emptyTrack: 'rgba(255, 255, 255, 0.015)',
    emptyStroke: 'rgba(255, 255, 255, 0.04)'
  };
 
  function updateVuColors() {
    if (!containerEl) return;
    const style = window.getComputedStyle(containerEl);
    vuColors = {
      bg: style.getPropertyValue('--vu-bg').trim() || '#0a0a12',
      divider: style.getPropertyValue('--vu-divider').trim() || '#0a0a12',
      text: style.getPropertyValue('--vu-text').trim() || 'rgba(148, 163, 184, 0.55)',
      emptyTrack: style.getPropertyValue('--vu-empty-track').trim() || 'rgba(255, 255, 255, 0.015)',
      emptyStroke: style.getPropertyValue('--vu-empty-stroke').trim() || 'rgba(255, 255, 255, 0.04)'
    };
  }
 
  /* ── Render ─────────────────────────────── */
  function render() {
    if (!ctx || !canvas) {
      animFrameId = requestAnimationFrame(render);
      return;
    }
 
    // Easing constants for professional analog feel
    const RISE_SMOOTH = 0.11; // Gentle exponential rise (mimics analog physical mass)
    const FALL_SMOOTH = 0.04; // Double-smoothed decay for silky ballistics
    const PEAK_SMOOTH = 0.05; // Gliding ease for peak indicator lines
 
    // Interpolate Left Channel
    const diffLeft = targetLeft - currentLeft;
    if (diffLeft > 0) {
      currentLeft += diffLeft * RISE_SMOOTH;
    } else {
      currentLeft += diffLeft * FALL_SMOOTH;
    }
 
    // Interpolate Right Channel
    const diffRight = targetRight - currentRight;
    if (diffRight > 0) {
      currentRight += diffRight * RISE_SMOOTH;
    } else {
      currentRight += diffRight * FALL_SMOOTH;
    }
 
    // Interpolate Peak Indicators
    peakLeft += (targetPeakLeft - peakLeft) * PEAK_SMOOTH;
    peakRight += (targetPeakRight - peakRight) * PEAK_SMOOTH;
 
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.width / dpr;
    const h = canvas.height / dpr;
 
    ctx.save();
    ctx.scale(dpr, dpr);
 
    // Background
    ctx.fillStyle = vuColors.bg;
    ctx.fillRect(0, 0, w, h);
 
    if (vertical) {
      // ── VERTICAL MODE ─────────────────────
      const padX = 6;
      const padY = 24;
      const gapX = 4;
      const barW = (w - padX * 2 - gapX) / 2;
      const meterH = h - padY - 18;
 
      // Draw grid lines
      for (const db of gridLines) {
        const frac = dbToFraction(db);
        const y = h - 18 - frac * meterH;
 
        ctx.strokeStyle = vuColors.emptyStroke;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(padX, y);
        ctx.lineTo(w - padX, y);
        ctx.stroke();
      }
 
      // Draw vertical bars
      function drawVerticalBar(x: number, level: number, peakHold: number, label: string) {
        const frac = dbToFraction(level);
        const barHeight = frac * meterH;
        const y = h - 18 - barHeight;
 
        // Background track (empty bar channel) - crisp rectangular well
        ctx!.fillStyle = vuColors.emptyTrack;
        ctx!.fillRect(x, h - 18 - meterH, barW, meterH);
        
        ctx!.strokeStyle = vuColors.emptyStroke;
        ctx!.lineWidth = 1;
        ctx!.strokeRect(x, h - 18 - meterH, barW, meterH);
 
        if (barHeight > 0) {
          const grad = ctx!.createLinearGradient(0, h - 18, 0, h - 18 - meterH);
          grad.addColorStop(0, '#22c55e'); // Green
          grad.addColorStop(dbToFraction(-12) || 0.8, '#22c55e');
          grad.addColorStop(dbToFraction(-6) || 0.9, '#eab308'); // Yellow
          grad.addColorStop(dbToFraction(-3) || 0.95, '#f59e0b'); // Orange
          grad.addColorStop(1, '#ef4444'); // Red
 
          ctx!.fillStyle = grad;
          ctx!.fillRect(x, y, barW, barHeight);
 
          // Sleek, high-intensity linear needle tip highlight
          ctx!.fillStyle = level > -4
            ? '#ffcaca' // bright light red
            : level > -12
              ? '#fff3ca' // bright light yellow
              : '#caffd8'; // bright light green
          ctx!.fillRect(x, y, barW, 1.5);
        }
 
        // Peak hold thin horizontal line
        const peakHoldFrac = dbToFraction(peakHold);
        if (peakHoldFrac > 0.001) {
          const peakY = h - 18 - peakHoldFrac * meterH;
          ctx!.strokeStyle = peakHold > -4
            ? 'rgba(255, 68, 68, 0.95)'
            : peakHold > -12
              ? 'rgba(255, 180, 11, 0.95)'
              : vuColors.text;
          ctx!.lineWidth = 2;
          ctx!.beginPath();
          ctx!.moveTo(x, peakY);
          ctx!.lineTo(x + barW, peakY);
          ctx!.stroke();
        }
 
        // Bottom label
        ctx!.textAlign = 'center';
        ctx!.textBaseline = 'top';
        ctx!.font = 'bold 9px Inter, sans-serif';
        ctx!.fillStyle = vuColors.text;
        ctx!.fillText(label, x + barW / 2, h - 13);
      }
 
      drawVerticalBar(padX, currentLeft, peakHoldLeft, 'L');
      drawVerticalBar(padX + barW + gapX, currentRight, peakHoldRight, 'R');
 
      // Draw grid line overlay dividers to segment the level bars with professional precision
      for (const db of gridLines) {
        const frac = dbToFraction(db);
        const y = h - 18 - frac * meterH;
 
        ctx.strokeStyle = vuColors.divider; // match background/divider to mask/segment
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(padX, y);
        ctx.lineTo(w - padX, y);
        ctx.stroke();
      }
 
      // Grid overlays (centered dB values)
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.font = '8px Inter, sans-serif';
      ctx.fillStyle = vuColors.text;
      for (const db of gridLines) {
        const frac = dbToFraction(db);
        const y = h - 18 - frac * meterH;
        ctx.fillText(db === 0 ? '0' : `${db}`, w / 2, y);
      }
 
    } else {
      // ── HORIZONTAL MODE ───────────────────
      const padX = 48;
      const padY = 4;
      const meterW = w - padX - 12;
      const gapY = 4;
      const barH = (h - padY * 2 - gapY) / 2;
 
      // Draw grid lines
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.font = '9px Inter, sans-serif';
 
      for (const db of gridLines) {
        const frac = dbToFraction(db);
        const x = padX + frac * meterW;
 
        ctx.strokeStyle = vuColors.emptyStroke;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(x, padY);
        ctx.lineTo(x, h - padY);
        ctx.stroke();
 
        ctx.fillStyle = vuColors.text;
        ctx.fillText(db === 0 ? '0' : `${db}`, x, h - padY + 2);
      }
 
      // Channel labels
      ctx.textAlign = 'right';
      ctx.textBaseline = 'middle';
      ctx.font = '10px Inter, sans-serif';
      ctx.fillStyle = vuColors.text;
      ctx.fillText('L', padX - 8, padY + barH / 2);
      ctx.fillText('R', padX - 8, padY + barH + gapY + barH / 2);
 
      // Draw bar function - crisp rectangular horizontal channel
      function drawBar(y: number, level: number, peakHold: number) {
        // Background track (empty bar channel) - sleek inset look
        ctx!.fillStyle = vuColors.emptyTrack;
        ctx!.fillRect(padX, y, meterW, barH);
        
        ctx!.strokeStyle = vuColors.emptyStroke;
        ctx!.lineWidth = 1;
        ctx!.strokeRect(padX, y, meterW, barH);
 
        const frac = dbToFraction(level);
        const barWidth = frac * meterW;
 
        if (barWidth > 0) {
          const grad = ctx!.createLinearGradient(padX, 0, padX + meterW, 0);
          grad.addColorStop(0, '#22c55e'); // Green
          grad.addColorStop(dbToFraction(-12) || 0.8, '#22c55e');
          grad.addColorStop(dbToFraction(-6) || 0.9, '#eab308'); // Yellow
          grad.addColorStop(dbToFraction(-3) || 0.95, '#f59e0b'); // Orange
          grad.addColorStop(1, '#ef4444'); // Red
 
          ctx!.fillStyle = grad;
          ctx!.fillRect(padX, y, barWidth, barH);
 
          // Sleek, high-intensity linear needle tip highlight
          ctx!.fillStyle = level > -4
            ? '#ffcaca' // bright light red
            : level > -12
              ? '#fff3ca' // bright light yellow
              : '#caffd8'; // bright light green
          ctx!.fillRect(padX + barWidth - 1.5, y, 1.5, barH);
        }
 
        const peakHoldFrac = dbToFraction(peakHold);
        if (peakHoldFrac > 0.001) {
          const peakX = padX + peakHoldFrac * meterW;
          ctx!.strokeStyle = peakHold > -4
            ? 'rgba(255, 68, 68, 0.95)'
            : peakHold > -12
              ? 'rgba(255, 180, 11, 0.95)'
              : vuColors.text;
          ctx!.lineWidth = 2.5;
          ctx!.beginPath();
          ctx!.moveTo(peakX, y - 0.5);
          ctx!.lineTo(peakX, y + barH + 0.5);
          ctx!.stroke();
        }
      }
 
      drawBar(padY, currentLeft, peakHoldLeft);
      drawBar(padY + barH + gapY, currentRight, peakHoldRight);
 
      // Draw grid line overlay dividers to segment the level bars with professional precision
      for (const db of gridLines) {
        const frac = dbToFraction(db);
        const x = padX + frac * meterW;
 
        ctx.strokeStyle = vuColors.divider; // match background/divider to mask/segment
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(x, padY);
        ctx.lineTo(x, h - padY - 12);
        ctx.stroke();
      }
 
      // Polish top/bottom border
      const borderGrad = ctx.createLinearGradient(0, 0, w, 0);
      borderGrad.addColorStop(0, 'transparent');
      borderGrad.addColorStop(0.3, 'rgba(59,130,246,0.15)');
      borderGrad.addColorStop(0.7, 'rgba(6,182,212,0.15)');
      borderGrad.addColorStop(1, 'transparent');
 
      ctx.strokeStyle = borderGrad;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(0, h - 0.5);
      ctx.lineTo(w, h - 0.5);
      ctx.stroke();
    }
 
    ctx.restore();
 
    // Peak hold decay logic
    const now = performance.now();
 
    if (peakLeft > peakHoldLeft) {
      peakHoldLeft = peakLeft;
      peakHoldLeftTimer = now;
    } else if (now - peakHoldLeftTimer > PEAK_HOLD_MS) {
      peakHoldLeft = Math.max(DB_MIN, peakHoldLeft - PEAK_DECAY_RATE);
    }
 
    if (peakRight > peakHoldRight) {
      peakHoldRight = peakRight;
      peakHoldRightTimer = now;
    } else if (now - peakHoldRightTimer > PEAK_HOLD_MS) {
      peakHoldRight = Math.max(DB_MIN, peakHoldRight - PEAK_DECAY_RATE);
    }
 
    animFrameId = requestAnimationFrame(render);
  }
 
  /* ── Resize canvas ──────────────────────── */
  function resizeCanvas() {
    if (!canvas || !containerEl) return;
    const dpr = window.devicePixelRatio || 1;
    const rect = containerEl.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;
  }
 
  /* ── Lifecycle ──────────────────────────── */
  onMount(() => {
    ctx = canvas.getContext('2d');
    resizeCanvas();
    updateVuColors();
 
    const resizeObs = new ResizeObserver(() => {
      resizeCanvas();
      updateVuColors();
    });
    resizeObs.observe(containerEl);
 
    // Listen to prefers-color-scheme queries to adapt canvas colors instantly
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleThemeChange = () => {
      setTimeout(updateVuColors, 20); // brief timeout to let CSS variables propagate
    };
    mediaQuery.addEventListener('change', handleThemeChange);
 
    animFrameId = requestAnimationFrame(render);
 
    const unlistenPromise = onVuMeter((data: VuMeterData) => {
      targetLeft = data.left;
      targetRight = data.right;
      targetPeakLeft = data.peak_left;
      targetPeakRight = data.peak_right;
    });
 
    return () => {
      cancelAnimationFrame(animFrameId);
      resizeObs.disconnect();
      mediaQuery.removeEventListener('change', handleThemeChange);
      unlistenPromise.then((fn) => fn());
    };
  });
</script>
 
<div bind:this={containerEl} class="vu-meter-container {vertical ? 'vertical' : ''}">
  <canvas bind:this={canvas}></canvas>
</div>
 
<style>
  .vu-meter-container {
    width: 100%;
    height: 64px;
    position: relative;
    border-radius: 2px;
    overflow: hidden;
    background: var(--vu-bg);
    box-shadow:
      inset 0 1px 0 var(--vu-empty-stroke),
      0 2px 12px rgba(0, 0, 0, 0.2);
  }
 
  .vu-meter-container.vertical {
    height: 100%;
    border-radius: 2px;
  }
 
  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
