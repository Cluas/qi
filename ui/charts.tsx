import { Canvas, colorScheme, Text, token, View } from "nsh";
import type { TrendPoint } from "../store";

function resolveStateColor(state: string): string {
  switch (state) {
    case "prime_ready":
      return token("vital-prime");
    case "moderate_steady":
      return token("vital-steady");
    default:
      return token("vital-recover");
  }
}

/** 今日元气 270° 宋韵仪表面环（原生 Canvas 绘制） */
export function VitalityRingCanvas(props: { score: number; state: string; badge: string }) {
  return (
    <Canvas
      style={{ width: 116, height: 116 }}
      draw={(cx, size) => {
        const scheme = colorScheme();
        const score = Math.max(0, Math.min(100, props.score));
        const activeColor = resolveStateColor(props.state);
        const trackColor = token("border");
        const fgColor = token("foreground");
        const mutedColor = token("muted-foreground");

        const cxMid = size.width / 2;
        const cyMid = size.height / 2 + 2;
        const radius = Math.min(size.width, size.height) / 2 - 11;
        const startAngle = Math.PI * 0.75;
        const sweepTotal = Math.PI * 1.5;
        const endAngle = startAngle + sweepTotal;
        const progressEnd = startAngle + (sweepTotal * score) / 100;

        cx.clearRect(0, 0, size.width, size.height);

        // 270° 开口底轨
        cx.save();
        cx.lineWidth = 10;
        cx.lineCap = "round";
        cx.strokeStyle = trackColor;
        cx.globalAlpha = scheme === "dark" ? 0.65 : 0.9;
        cx.beginPath();
        cx.arc(cxMid, cyMid, radius, startAngle, endAngle);
        cx.stroke();
        cx.restore();

        // 活跃元气圆弧
        if (score > 0) {
          cx.save();
          cx.lineWidth = 10;
          cx.lineCap = "round";
          cx.strokeStyle = activeColor;
          cx.beginPath();
          cx.arc(cxMid, cyMid, radius, startAngle, progressEnd);
          cx.stroke();
          cx.restore();
        }

        // 中心等宽数字评分
        cx.save();
        cx.textAlign = "center";
        cx.textBaseline = "middle";
        cx.fillStyle = fgColor;
        cx.font = "700 28px PingFang SC";
        cx.fontVariantNumeric = "tabular-nums";
        cx.fillText(`${Math.round(score)}`, cxMid, cyMid - 4);

        cx.fillStyle = mutedColor;
        cx.font = "600 10px PingFang SC";
        cx.fillText(props.badge, cxMid, cyMid + 22);
        cx.restore();
      }}
    />
  );
}

/** ACWR (7d/28d) 三区间负荷轨道指示器（0.8 ~ 1.3 为黄金甜区） */
export function AcwrZoneBarCanvas(props: { acwr: number | null }) {
  return (
    <Canvas
      class="w-full"
      style={{ height: 44 }}
      draw={(cx, size) => {
        const w = Math.max(180, size.width);
        const h = size.height;
        const padX = 8;
        const trackW = w - padX * 2;
        const barY = 10;
        const barH = 12;

        cx.clearRect(0, 0, w, h);

        // 区间比例按 [0.0, 2.0] 线性映射：0.8 = 40%，1.3 = 65%
        const warmW = trackW * 0.4;
        const sweetW = trackW * 0.25;
        const overW = trackW - warmW - sweetW;

        // 1) 热身不足区 0.0 - 0.8
        cx.save();
        cx.fillStyle = token("vital-steady-soft");
        cx.beginPath();
        cx.roundRect(padX, barY, warmW - 2, barH, [6, 2, 2, 6]);
        cx.fill();

        // 2) 黄金甜区 0.8 - 1.3
        cx.fillStyle = token("vital-prime-soft");
        cx.strokeStyle = token("vital-prime");
        cx.lineWidth = 1.25;
        cx.beginPath();
        cx.roundRect(padX + warmW, barY, sweetW - 2, barH, 3);
        cx.fill();
        cx.stroke();

        // 3) 过载警戒区 1.3 - 2.0
        cx.fillStyle = token("vital-recover-soft");
        cx.beginPath();
        cx.roundRect(padX + warmW + sweetW, barY, overW, barH, [2, 6, 6, 2]);
        cx.fill();
        cx.restore();

        // 刻度文字
        cx.save();
        cx.font = "500 10px PingFang SC";
        cx.textBaseline = "top";
        cx.fillStyle = token("muted-foreground");
        cx.textAlign = "center";
        cx.fillText("0.8 甜区", padX + warmW, barY + barH + 5);
        cx.fillText("1.3 警戒", padX + warmW + sweetW, barY + barH + 5);
        cx.restore();

        // 当前 ACWR 游标圆点
        if (props.acwr !== null) {
          const clamped = Math.max(0.05, Math.min(1.95, props.acwr));
          const ratio = clamped / 2.0;
          const markerX = padX + trackW * ratio;
          const markerY = barY + barH / 2;
          const dotColor =
            clamped < 0.8
              ? token("vital-steady")
              : clamped <= 1.3
                ? token("vital-prime")
                : token("vital-recover");

          cx.save();
          cx.fillStyle = token("card");
          cx.beginPath();
          cx.arc(markerX, markerY, 7, 0, Math.PI * 2);
          cx.fill();

          cx.fillStyle = dotColor;
          cx.beginPath();
          cx.arc(markerX, markerY, 4.5, 0, Math.PI * 2);
          cx.fill();
          cx.restore();
        }
      }}
    />
  );
}

/** 14 天动态节律平滑面积折线图 */
export function RhythmSparklineCanvas(props: { points: TrendPoint[] }) {
  return (
    <View class="w-full">
      <If when={props.points.length === 0}>
        <Text class="text-xs text-muted-foreground">暂无 14 天节律序列</Text>
      </If>
      <If when={props.points.length > 0}>
        <Canvas
          class="w-full"
          style={{ height: 128 }}
          draw={(cx, size) => {
            const pts = props.points;
            if (pts.length === 0) return;

            const w = Math.max(200, size.width);
            const h = size.height;
            const padL = 26;
            const padR = 12;
            const padT = 12;
            const padB = 22;
            const plotW = w - padL - padR;
            const plotH = h - padT - padB;

            cx.clearRect(0, 0, w, h);

            const yFor = (val: number) => {
              const clamped = Math.max(0, Math.min(100, val));
              return padT + plotH * (1 - clamped / 100);
            };
            const xFor = (idx: number) => {
              if (pts.length === 1) return padL + plotW / 2;
              return padL + (plotW * idx) / (pts.length - 1);
            };

            // 80 与 50 虚线参考基线
            cx.save();
            cx.strokeStyle = token("border");
            cx.lineWidth = 1;
            cx.setLineDash([4, 4]);
            for (const level of [80, 50]) {
              const y = yFor(level);
              cx.beginPath();
              cx.moveTo(padL, y);
              cx.lineTo(w - padR, y);
              cx.stroke();

              cx.fillStyle = token("muted-foreground");
              cx.font = "500 9px SF Mono";
              cx.textAlign = "right";
              cx.textBaseline = "middle";
              cx.fillText(`${level}`, padL - 5, y);
            }
            cx.restore();

            const coords = pts.map((p, i) => ({
              x: xFor(i),
              y: yFor(p.qi_score),
              state: p.state,
              label: p.short_label,
            }));

            // 平滑渐变填充区域
            if (coords.length >= 2) {
              cx.save();
              cx.fillStyle = token("vital-prime");
              cx.globalAlpha = colorScheme() === "dark" ? 0.18 : 0.12;
              cx.beginPath();
              cx.moveTo(coords[0].x, padT + plotH);
              cx.lineTo(coords[0].x, coords[0].y);
              for (let i = 0; i < coords.length - 1; i++) {
                const curr = coords[i];
                const next = coords[i + 1];
                const midX = (curr.x + next.x) / 2;
                const midY = (curr.y + next.y) / 2;
                cx.quadraticCurveTo(curr.x, curr.y, midX, midY);
              }
              const last = coords[coords.length - 1];
              cx.lineTo(last.x, last.y);
              cx.lineTo(last.x, padT + plotH);
              cx.closePath();
              cx.fill();
              cx.restore();

              // 平滑主折线
              cx.save();
              cx.strokeStyle = token("primary");
              cx.lineWidth = 2.25;
              cx.lineCap = "round";
              cx.lineJoin = "round";
              cx.beginPath();
              cx.moveTo(coords[0].x, coords[0].y);
              for (let i = 0; i < coords.length - 1; i++) {
                const curr = coords[i];
                const next = coords[i + 1];
                const midX = (curr.x + next.x) / 2;
                const midY = (curr.y + next.y) / 2;
                cx.quadraticCurveTo(curr.x, curr.y, midX, midY);
              }
              cx.lineTo(last.x, last.y);
              cx.stroke();
              cx.restore();
            }

            // 节点圆点 + 每隔 2 天底部日期刻度
            cx.save();
            cx.font = "500 9px SF Mono";
            cx.textAlign = "center";
            cx.textBaseline = "top";
            for (let i = 0; i < coords.length; i++) {
              const c = coords[i];
              cx.fillStyle = token("card");
              cx.beginPath();
              cx.arc(c.x, c.y, 4.2, 0, Math.PI * 2);
              cx.fill();

              cx.fillStyle = resolveStateColor(c.state);
              cx.beginPath();
              cx.arc(c.x, c.y, 2.8, 0, Math.PI * 2);
              cx.fill();

              if (i === 0 || i === coords.length - 1 || i % 3 === 0) {
                cx.fillStyle = token("muted-foreground");
                cx.fillText(c.label.slice(3), c.x, padT + plotH + 6);
              }
            }
            cx.restore();
          }}
        />
      </If>
    </View>
  );
}
