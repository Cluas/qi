import { Canvas, colorScheme, signal, Text, token, View } from "nsh";
import { Area, Grid, Line, Plot, PlotAxis } from "nsh/base/plot";
import type { TrendPoint } from "../store";

export type RhythmPoint = TrendPoint & {
  hrv_ms?: number | null;
  sleep_h?: number | null;
};

function readToken(name: string): string {
  return (token as (key: string) => string)(name);
}

function resolveStateColor(state: string): string {
  switch (state) {
    case "prime_ready":
    case "optimal":
    case "normal":
      return token("vital-prime");
    case "moderate_steady":
    case "watch_low":
    case "watch_high":
    case "shifted":
    case "low":
      return token("vital-steady");
    default:
      return token("vital-recover");
  }
}

function resolveLabFlagColor(flag: string): string {
  switch (flag) {
    case "normal":
    case "optimal":
      return token("vital-prime");
    case "borderline":
    case "watch_low":
    case "watch_high":
      return token("vital-steady");
    case "high":
    case "low":
    case "alert_high":
    case "alert_low":
      return token("vital-recover");
    default:
      return token("primary");
  }
}

/** 今日元气 136×136 双层宋韵瓷盘仪表弧（外环刻度点 + 内环柔润圆弧） */
export function VitalityDialCanvas(props: { score: number; state: string; badge: string }) {
  return (
    <Canvas
      style={{ width: 136, height: 136 }}
      draw={(cx, size) => {
        const scheme = colorScheme();
        const score = Math.max(0, Math.min(100, Number.isFinite(props.score) ? props.score : 0));
        const activeColor = resolveStateColor(props.state);
        const trackColor = token("border");
        const subtleInk = readToken("ink-subtle");
        const fgColor = token("foreground");

        const cxMid = size.width / 2;
        const cyMid = size.height / 2 + 3;
        const outerRadius = Math.min(size.width, size.height) / 2 - 7;
        const innerRadius = outerRadius - 12;
        const startAngle = Math.PI * 0.75;
        const sweepTotal = Math.PI * 1.5;
        const endAngle = startAngle + sweepTotal;
        const progressEnd = startAngle + (sweepTotal * score) / 100;

        cx.clearRect(0, 0, size.width, size.height);

        // 1) 外环 29 枚宋瓷刻度点
        const tickCount = 29;
        for (let i = 0; i < tickCount; i++) {
          const ratio = i / (tickCount - 1);
          const angle = startAngle + sweepTotal * ratio;
          const tx = cxMid + Math.cos(angle) * outerRadius;
          const ty = cyMid + Math.sin(angle) * outerRadius;
          const isLit = ratio * 100 <= score && score > 0;

          cx.save();
          cx.fillStyle = isLit ? activeColor : subtleInk;
          cx.globalAlpha = isLit ? 0.85 : scheme === "dark" ? 0.35 : 0.48;
          cx.beginPath();
          cx.arc(tx, ty, isLit ? 1.8 : 1.3, 0, Math.PI * 2);
          cx.fill();
          cx.restore();
        }

        // 2) 内环柔润底轨
        cx.save();
        cx.lineWidth = 10;
        cx.lineCap = "round";
        cx.strokeStyle = trackColor;
        cx.globalAlpha = scheme === "dark" ? 0.55 : 0.85;
        cx.beginPath();
        cx.arc(cxMid, cyMid, innerRadius, startAngle, endAngle);
        cx.stroke();
        cx.restore();

        // 3) 内环活跃元气圆弧
        if (score > 0) {
          cx.save();
          cx.lineWidth = 10;
          cx.lineCap = "round";
          cx.strokeStyle = activeColor;
          cx.beginPath();
          cx.arc(cxMid, cyMid, innerRadius, startAngle, progressEnd);
          cx.stroke();
          cx.restore();
        }

        // 4) 内瓷盘微细饰线
        cx.save();
        cx.lineWidth = 1;
        cx.strokeStyle = trackColor;
        cx.globalAlpha = scheme === "dark" ? 0.25 : 0.45;
        cx.beginPath();
        cx.arc(cxMid, cyMid, innerRadius - 9, 0, Math.PI * 2);
        cx.stroke();
        cx.restore();

        // 5) 中心大号等宽数字与状态文案
        cx.save();
        cx.textAlign = "center";
        cx.textBaseline = "middle";
        cx.fillStyle = fgColor;
        cx.font = "700 32px PingFang SC";
        cx.fontVariantNumeric = "tabular-nums";
        cx.fillText(`${Math.round(score)}`, cxMid, cyMid - 5);

        cx.fillStyle = activeColor;
        cx.font = "600 11px PingFang SC";
        cx.fillText(props.badge, cxMid, cyMid + 24);
        cx.restore();
      }}
    />
  );
}

/** 兼容既有页面导入别名 */
export const VitalityRingCanvas = VitalityDialCanvas;

/** 交互式近 14 天元气节律图（顶部选中日读数栏 + nsh/base/plot 原生平滑面积曲线） */
export function InteractiveRhythmPlot(props: { points: RhythmPoint[] }) {
  const hoveredIdx = signal<number | null>(null);

  const activePoint = (): RhythmPoint | null => {
    const pts = props.points;
    if (pts.length === 0) return null;
    const idx = hoveredIdx();
    if (idx !== null && idx >= 0 && idx < pts.length) {
      return pts[idx];
    }
    return pts[pts.length - 1];
  };

  return (
    <View class="w-full gap-2.5">
      <If
        when={props.points.length > 0}
        else={
          <View class="items-center justify-center rounded-xl border border-border/60 bg-surface-sunken py-7">
            <Text class="text-xs text-muted-foreground">暂无近期节律变化数据</Text>
          </View>
        }
      >
        <If when={activePoint()}>
          {(pt) => (
            <View class="flex-row items-center justify-between gap-2 rounded-xl border border-border/60 bg-surface-sunken px-3 py-2">
              <View class="flex-row items-center gap-2">
                <Text class="text-xs font-semibold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                  {pt().short_label || pt().date}
                </Text>
                <Text class="text-[11px] text-muted-foreground">
                  {hoveredIdx() !== null ? "选中日读数" : "最新日读数"}
                </Text>
              </View>
              <View class="flex-row items-center gap-2.5">
                <Text class="text-xs font-bold text-primary" style={{ fontVariant: ["tabular-nums"] }}>
                  {`元气 ${Math.round(pt().qi_score)}`}
                </Text>
                <If when={pt().hrv_ms !== undefined && pt().hrv_ms !== null}>
                  <Text class="text-[11px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                    {`HRV ${Math.round(pt().hrv_ms!)}ms`}
                  </Text>
                </If>
                <If when={pt().sleep_h !== undefined && pt().sleep_h !== null}>
                  <Text class="text-[11px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                    {`睡眠 ${pt().sleep_h!.toFixed(1)}h`}
                  </Text>
                </If>
              </View>
            </View>
          )}
        </If>

        <View class="pb-5 pl-7 pr-2 pt-2">
          <Plot<RhythmPoint>
            data={props.points}
            x={(_d, index) => index}
            y={(d) => d.qi_score}
            xScale={{ type: "point" }}
            yScale={{ type: "linear", domain: [0, 100] }}
            interactive={true}
            class="h-32 w-full"
            crossLineClass="w-px bg-primary/45"
            dotClass="size-2.5 rounded-full border-2 border-card bg-primary"
            onHoverChange={(event) => {
              hoveredIdx.set(event.index);
            }}
          >
            <Grid
              y={[25, 50, 75]}
              class="stroke-border"
              style={{ strokeWidth: 1, strokeDasharray: "3 3" }}
            />
            <Area
              curve="natural"
              y0={0}
              backgroundClass="bg-linear-to-b from-primary/25 to-primary/0"
              hoverDotClass="hidden"
            />
            <Line
              curve="natural"
              class="stroke-primary"
              style={{ strokeWidth: 2.25 }}
            />
            <PlotAxis
              placement="left"
              tickValues={[25, 50, 75, 100]}
              labelClass="pr-1.5 text-[10px] text-muted-foreground"
            />
            <PlotAxis
              placement="bottom"
              ticks={4}
              format={(value) => {
                const idx = Math.round(value);
                const pt = props.points[idx];
                return pt ? pt.short_label : "";
              }}
              labelClass="pt-1.5 text-[10px] text-muted-foreground"
            />
          </Plot>
        </View>
      </If>
    </View>
  );
}

/** 兼容既有页面导入的 14 天动态节律曲线组件 */
export function RhythmSparklineCanvas(props: { points: TrendPoint[] }) {
  return <InteractiveRhythmPlot points={props.points} />;
}

/** 四期睡眠结构分段条（深睡 / 浅睡 / 快速眼动 / 清醒）+ 下方图例分钟数 */
export function SleepArchitectureBar(props: {
  deepMin: number;
  coreMin: number;
  remMin: number;
  awakeMin: number;
}) {
  const clean = (n: number) => (Number.isFinite(n) && n > 0 ? Math.round(n) : 0);
  const deep = () => clean(props.deepMin);
  const core = () => clean(props.coreMin);
  const rem = () => clean(props.remMin);
  const awake = () => clean(props.awakeMin);
  const total = () => deep() + core() + rem() + awake();

  const legendItems = () => [
    { key: "deep", label: "深睡", min: deep(), tokenName: "sleep-deep" },
    { key: "core", label: "浅睡", min: core(), tokenName: "sleep-core" },
    { key: "rem", label: "眼动", min: rem(), tokenName: "sleep-rem" },
    { key: "awake", label: "微醒", min: awake(), tokenName: "sleep-awake" },
  ];

  return (
    <View class="w-full gap-2.5">
      <If
        when={total() > 0}
        else={
          <View class="items-center justify-center rounded-xl border border-border/60 bg-surface-sunken py-3">
            <Text class="text-xs text-muted-foreground">暂无睡眠分期记录</Text>
          </View>
        }
      >
        <Canvas
          class="w-full"
          style={{ height: 18 }}
          draw={(cx, size) => {
            const w = Math.max(120, size.width);
            const h = size.height;
            const barY = 2;
            const barH = h - 4;

            const stages = [
              { val: deep(), color: readToken("sleep-deep") },
              { val: core(), color: readToken("sleep-core") },
              { val: rem(), color: readToken("sleep-rem") },
              { val: awake(), color: readToken("sleep-awake") },
            ].filter((s) => s.val > 0);

            cx.clearRect(0, 0, w, h);
            if (stages.length === 0) return;

            const gap = 2;
            const totalGaps = Math.max(0, stages.length - 1) * gap;
            const availW = Math.max(20, w - totalGaps);
            const sum = stages.reduce((acc, s) => acc + s.val, 0);

            cx.save();
            cx.beginPath();
            cx.roundRect(0, barY, w, barH, barH / 2);
            cx.clip();

            let cursorX = 0;
            for (let i = 0; i < stages.length; i++) {
              const segW =
                i === stages.length - 1
                  ? Math.max(0, w - cursorX)
                  : Math.max(4, (availW * stages[i].val) / sum);

              cx.fillStyle = stages[i].color;
              cx.fillRect(cursorX, barY, segW, barH);
              cursorX += segW + gap;
            }
            cx.restore();
          }}
        />

        <View class="flex-row flex-wrap items-center justify-between gap-2">
          <Each of={legendItems()} key={(item) => item.key}>
            {(item) => (
              <View class="flex-row items-center gap-1.5">
                <Canvas
                  style={{ width: 8, height: 8 }}
                  draw={(cx, s) => {
                    cx.clearRect(0, 0, s.width, s.height);
                    cx.fillStyle = readToken(item().tokenName);
                    cx.beginPath();
                    cx.arc(s.width / 2, s.height / 2, 3.5, 0, Math.PI * 2);
                    cx.fill();
                  }}
                />
                <Text class="text-xs text-muted-foreground">{item().label}</Text>
                <Text class="text-xs font-semibold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                  {`${item().min}m`}
                </Text>
              </View>
            )}
          </Each>
        </View>
      </If>
    </View>
  );
}

/** 体检指标临床参考区间子弹图（绿色安全参考带 + 上期空心参考圆圈 + 本期高对比实心圆点） */
export function LabRangeBulletCanvas(props: {
  value: number;
  refLow: number | null;
  refHigh: number | null;
  flag: string;
  prevValue?: number | null;
}) {
  return (
    <Canvas
      class="w-full"
      style={{ height: 48 }}
      draw={(cx, size) => {
        const w = Math.max(180, size.width);
        const h = size.height;
        const padX = 12;
        const trackW = w - padX * 2;
        const barY = 14;
        const barH = 8;
        const midY = barY + barH / 2;

        cx.clearRect(0, 0, w, h);

        const nums: number[] = [props.value];
        if (props.prevValue !== undefined && props.prevValue !== null && Number.isFinite(props.prevValue)) {
          nums.push(props.prevValue);
        }
        if (props.refLow !== null && Number.isFinite(props.refLow)) nums.push(props.refLow);
        if (props.refHigh !== null && Number.isFinite(props.refHigh)) nums.push(props.refHigh);

        const rawMin = Math.min(...nums);
        const rawMax = Math.max(...nums);
        const baseSpan = rawMax - rawMin;
        const span = baseSpan > 1e-6 ? baseSpan : Math.max(1, Math.abs(rawMin) * 0.3);
        const domainMin = rawMin - span * 0.18;
        const domainMax = rawMax + span * 0.18;

        const xFor = (v: number) => {
          const ratio = (v - domainMin) / (domainMax - domainMin);
          const clamped = Math.max(0, Math.min(1, ratio));
          return padX + trackW * clamped;
        };

        // 1) 背景中性轨道
        cx.save();
        cx.fillStyle = token("surface-sunken");
        cx.strokeStyle = token("border");
        cx.lineWidth = 1;
        cx.beginPath();
        cx.roundRect(padX, barY, trackW, barH, barH / 2);
        cx.fill();
        cx.stroke();
        cx.restore();

        // 2) 临床参考安全区间带（绿色半透）
        const hasLow = props.refLow !== null && Number.isFinite(props.refLow);
        const hasHigh = props.refHigh !== null && Number.isFinite(props.refHigh);
        if (hasLow || hasHigh) {
          const bandStart = hasLow ? xFor(props.refLow!) : padX;
          const bandEnd = hasHigh ? xFor(props.refHigh!) : padX + trackW;
          const bandW = Math.max(4, bandEnd - bandStart);

          cx.save();
          cx.fillStyle = token("vital-prime-soft");
          cx.strokeStyle = token("vital-prime");
          cx.lineWidth = 1.25;
          cx.beginPath();
          cx.roundRect(bandStart, barY, bandW, barH, 4);
          cx.fill();
          cx.stroke();
          cx.restore();

          // 参考边界刻度数字
          cx.save();
          cx.font = "500 9px SF Mono";
          cx.fontVariantNumeric = "tabular-nums";
          cx.fillStyle = readToken("ink-subtle");
          cx.textBaseline = "top";
          if (hasLow) {
            cx.textAlign = "center";
            cx.fillText(`${props.refLow}`, bandStart, barY + barH + 7);
          }
          if (hasHigh) {
            cx.textAlign = "center";
            cx.fillText(`${props.refHigh}`, bandEnd, barY + barH + 7);
          }
          cx.restore();
        }

        const currX = xFor(props.value);
        const hasPrev =
          props.prevValue !== undefined &&
          props.prevValue !== null &&
          Number.isFinite(props.prevValue);

        // 3) 上期 -> 本期连线与上期空心参考圆圈
        if (hasPrev) {
          const prevX = xFor(props.prevValue!);
          cx.save();
          cx.strokeStyle = token("muted-foreground");
          cx.globalAlpha = 0.55;
          cx.lineWidth = 1.5;
          cx.setLineDash([2, 2]);
          cx.beginPath();
          cx.moveTo(prevX, midY);
          cx.lineTo(currX, midY);
          cx.stroke();
          cx.restore();

          cx.save();
          cx.fillStyle = token("card");
          cx.strokeStyle = token("muted-foreground");
          cx.lineWidth = 1.75;
          cx.beginPath();
          cx.arc(prevX, midY, 4.5, 0, Math.PI * 2);
          cx.fill();
          cx.stroke();
          cx.restore();
        }

        // 4) 本期高对比实心圆点（带卡片底色外晕环）
        const currColor = resolveLabFlagColor(props.flag);
        cx.save();
        cx.fillStyle = token("card");
        cx.beginPath();
        cx.arc(currX, midY, 7, 0, Math.PI * 2);
        cx.fill();

        cx.fillStyle = currColor;
        cx.beginPath();
        cx.arc(currX, midY, 4.8, 0, Math.PI * 2);
        cx.fill();
        cx.restore();
      }}
    />
  );
}

/** 运动负荷甜区轨道（大众化语言标注：恢复期 / 黄金适度区 / 偏累预警） */
export function AcwrSweetSpotCanvas(props: { acwr: number | null }) {
  return (
    <Canvas
      class="w-full"
      style={{ height: 48 }}
      draw={(cx, size) => {
        const w = Math.max(180, size.width);
        const h = size.height;
        const padX = 8;
        const trackW = w - padX * 2;
        const barY = 10;
        const barH = 12;

        cx.clearRect(0, 0, w, h);

        // 三段区间按 [0.0, 2.0] 映射：0.8 = 40%，1.3 = 65%
        const warmW = trackW * 0.4;
        const sweetW = trackW * 0.25;
        const overW = trackW - warmW - sweetW;

        // 1) 恢复期 (0.0 - 0.8)
        cx.save();
        cx.fillStyle = token("vital-steady-soft");
        cx.beginPath();
        cx.roundRect(padX, barY, warmW - 2, barH, [6, 2, 2, 6]);
        cx.fill();

        // 2) 黄金适度区 (0.8 - 1.3)
        cx.fillStyle = token("vital-prime-soft");
        cx.strokeStyle = token("vital-prime");
        cx.lineWidth = 1.25;
        cx.beginPath();
        cx.roundRect(padX + warmW, barY, sweetW - 2, barH, 3);
        cx.fill();
        cx.stroke();

        // 3) 偏累预警 (1.3 - 2.0)
        cx.fillStyle = token("vital-recover-soft");
        cx.beginPath();
        cx.roundRect(padX + warmW + sweetW, barY, overW, barH, [2, 6, 6, 2]);
        cx.fill();
        cx.restore();

        // 大众化区间文案
        cx.save();
        cx.font = "500 10px PingFang SC";
        cx.textBaseline = "top";
        cx.textAlign = "center";

        cx.fillStyle = token("muted-foreground");
        cx.fillText("恢复期", padX + warmW / 2, barY + barH + 6);

        cx.fillStyle = token("vital-prime");
        cx.font = "600 10px PingFang SC";
        cx.fillText("黄金适度区", padX + warmW + sweetW / 2, barY + barH + 6);

        cx.fillStyle = token("muted-foreground");
        cx.font = "500 10px PingFang SC";
        cx.fillText("偏累预警", padX + warmW + sweetW + overW / 2, barY + barH + 6);
        cx.restore();

        // 当前负荷游标圆点
        if (props.acwr !== null && Number.isFinite(props.acwr)) {
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
          cx.arc(markerX, markerY, 7.5, 0, Math.PI * 2);
          cx.fill();

          cx.fillStyle = dotColor;
          cx.beginPath();
          cx.arc(markerX, markerY, 4.8, 0, Math.PI * 2);
          cx.fill();
          cx.restore();
        }
      }}
    />
  );
}

/** 兼容既有页面导入别名 */
export const AcwrZoneBarCanvas = AcwrSweetSpotCanvas;
