import { Canvas, colorScheme, signal, Text, token, View } from "nsh";
import { Button } from "nsh/base";
import { Area, Grid, Line, Plot, PlotAxis } from "nsh/base/plot";
import type { TrendPoint } from "../store";
import { isEnglishLocale, tr } from "./i18n";

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

        // 5) 中心大号等宽数字与满分刻度（状态印章由卡片右侧独占承载，避免重复）
        cx.save();
        cx.textAlign = "center";
        cx.textBaseline = "middle";
        cx.fillStyle = fgColor;
        cx.font = '700 34px "PingFang SC", -apple-system, sans-serif';
        cx.fontVariantNumeric = "tabular-nums";
        cx.fillText(`${Math.round(score)}`, cxMid, cyMid - 5);

        cx.fillStyle = subtleInk;
        cx.font = '500 11px "Menlo", monospace';
        cx.fillText("/ 100", cxMid, cyMid + 22);
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
            <Text class="text-xs text-muted-foreground">{tr("charts.no_rhythm_data")}</Text>
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
                  {hoveredIdx() !== null ? tr("charts.selected_day") : tr("charts.latest_day")}
                </Text>
              </View>
              <View class="flex-row items-center gap-2.5">
                <Text class="text-xs font-bold text-primary" style={{ fontVariant: ["tabular-nums"] }}>
                  {tr("charts.vitality_readout", { score: Math.round(pt().qi_score) })}
                </Text>
                <If when={pt().hrv_ms !== undefined && pt().hrv_ms !== null}>
                  <Text class="text-[11px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                    {`HRV ${Math.round(pt().hrv_ms!)}ms`}
                  </Text>
                </If>
                <If when={pt().sleep_h !== undefined && pt().sleep_h !== null}>
                  <Text class="text-[11px] text-muted-foreground" style={{ fontVariant: ["tabular-nums"] }}>
                    {tr("charts.sleep_readout", { hours: pt().sleep_h!.toFixed(1) })}
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
    { key: "deep", label: tr("charts.deep_sleep"), min: deep(), tokenName: "sleep-deep" },
    { key: "core", label: tr("charts.light_sleep"), min: core(), tokenName: "sleep-core" },
    { key: "rem", label: tr("charts.rem_sleep"), min: rem(), tokenName: "sleep-rem" },
    { key: "awake", label: tr("charts.awake"), min: awake(), tokenName: "sleep-awake" },
  ];

  return (
    <View class="w-full gap-2.5">
      <If
        when={total() > 0}
        else={
          <View class="items-center justify-center rounded-xl border border-border/60 bg-surface-sunken py-3">
            <Text class="text-xs text-muted-foreground">{tr("charts.no_sleep_stages")}</Text>
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

        <View class="flex-row flex-wrap items-center justify-between gap-x-2 gap-y-1.5">
          <Each of={legendItems()} key={(item) => item.key}>
            {(item) => (
              <View class="flex-row items-center gap-1">
                <Canvas
                  style={{ width: 7, height: 7 }}
                  draw={(cx, s) => {
                    cx.clearRect(0, 0, s.width, s.height);
                    cx.fillStyle = readToken(item().tokenName);
                    cx.beginPath();
                    cx.arc(s.width / 2, s.height / 2, 3, 0, Math.PI * 2);
                    cx.fill();
                  }}
                />
                <Text class="text-[11px] text-muted-foreground">{item().label}</Text>
                <Text class="text-[11px] font-semibold text-foreground" style={{ fontVariant: ["tabular-nums"] }}>
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
          cx.font = '500 9px "Menlo", monospace';
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
        const axisFont500 = isEnglishLocale()
          ? "500 10px -apple-system, sans-serif"
          : '500 10px "PingFang SC", -apple-system, sans-serif';
        const axisFont600 = isEnglishLocale()
          ? "600 10px -apple-system, sans-serif"
          : '600 10px "PingFang SC", -apple-system, sans-serif';
        cx.save();
        cx.font = axisFont500;
        cx.textBaseline = "top";
        cx.textAlign = "center";

        cx.fillStyle = token("muted-foreground");
        cx.fillText(tr("charts.recovery_zone"), padX + warmW / 2, barY + barH + 6);

        cx.fillStyle = token("vital-prime");
        cx.font = axisFont600;
        cx.fillText(tr("charts.sweet_spot"), padX + warmW + sweetW / 2, barY + barH + 6);

        cx.fillStyle = token("muted-foreground");
        cx.font = axisFont500;
        cx.fillText(tr("charts.overreach_alert"), padX + warmW + sweetW + overW / 2, barY + barH + 6);
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

export interface UlnProfilePoint {
  date: string;
  short_label?: string;
  uln_pct: number;
  raw_value?: number;
  unit?: string;
}

export interface UlnProfileSeries {
  profile_id: string;
  label: string;
  points: UlnProfilePoint[];
}

function resolveProfileToken(profileId: string, idx: number): string {
  switch (profileId) {
    case "self":
      return readToken("vital-prime");
    case "partner":
      return readToken("duo-accent");
    case "parents":
      return readToken("ink-lapis");
    default:
      return idx === 0
        ? readToken("vital-prime")
        : idx === 1
          ? readToken("duo-accent")
          : idx === 2
            ? readToken("ink-lapis")
            : readToken("atelier-accent");
  }
}

/** 多亲友化验 %ULN 归一化趋势对比曲线（含 100% ULN 虚线基准线 + 档案胶囊筛选 + 节点读数） */
export function UlnTrajectoryCanvas(props: {
  series: UlnProfileSeries[];
  markerName?: string;
  activeProfileIds?: string[];
}) {
  const hiddenIds = signal<string[]>([]);
  const selectedIndex = signal<number | null>(null);

  const visibleSeries = () => {
    const base = props.activeProfileIds?.length
      ? props.series.filter((s) => props.activeProfileIds!.includes(s.profile_id))
      : props.series;
    const hidden = hiddenIds();
    const filtered = base.filter((s) => !hidden.includes(s.profile_id));
    return filtered.length > 0 ? filtered : base.slice(0, 1);
  };

  const timelineLabels = (): string[] => {
    let longest: UlnProfilePoint[] = [];
    for (const s of visibleSeries()) {
      if (s.points.length > longest.length) longest = s.points;
    }
    return longest.map((p) => p.short_label || p.date);
  };

  const activePointIndex = () => {
    const labels = timelineLabels();
    if (labels.length === 0) return -1;
    const idx = selectedIndex();
    if (idx !== null && idx >= 0 && idx < labels.length) return idx;
    return labels.length - 1;
  };

  const toggleProfile = (id: string) => {
    const current = hiddenIds();
    if (current.includes(id)) {
      hiddenIds.set(current.filter((item) => item !== id));
      return;
    }
    const visibleCount = props.series.filter((s) => !current.includes(s.profile_id)).length;
    if (visibleCount <= 1) return;
    hiddenIds.set([...current, id]);
  };

  return (
    <View class="w-full gap-2.5">
      <View class="flex-row flex-wrap items-center justify-between gap-2">
        <View class="flex-row flex-wrap items-center gap-1.5">
          <Each of={props.series} key={(s) => s.profile_id}>
            {(item, idx) => {
              const isActive = () =>
                visibleSeries().some((v) => v.profile_id === item().profile_id);
              return (
                <Button
                  hitSlop={4}
                  onPress={() => toggleProfile(item().profile_id)}
                  class={`min-h-8 flex-row items-center gap-1.5 rounded-full border px-3 py-1 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                    isActive()
                      ? "border-primary/45 bg-card"
                      : "border-border/60 bg-surface-sunken opacity-55"
                  }`}
                >
                  <Canvas
                    style={{ width: 8, height: 8 }}
                    draw={(cx, size) => {
                      cx.clearRect(0, 0, size.width, size.height);
                      cx.fillStyle = resolveProfileToken(item().profile_id, idx());
                      cx.beginPath();
                      cx.arc(size.width / 2, size.height / 2, 3.5, 0, Math.PI * 2);
                      cx.fill();
                    }}
                  />
                  <Text class="text-xs font-semibold text-foreground">{item().label}</Text>
                </Button>
              );
            }}
          </Each>
        </View>
        <If when={Boolean(props.markerName)}>
          <Text class="text-[11px] font-medium text-muted-foreground">
            {tr("charts.marker_uln_suffix", { marker: props.markerName ?? "" })}
          </Text>
        </If>
      </View>

      <If
        when={timelineLabels().length > 0}
        else={
          <View class="items-center justify-center rounded-xl border border-border/60 bg-surface-sunken py-7">
            <Text class="text-xs text-muted-foreground">{tr("charts.no_lab_trajectory")}</Text>
          </View>
        }
      >
        <View class="flex-row flex-wrap items-center justify-between gap-2 rounded-xl border border-border/60 bg-surface-sunken px-3 py-2">
          <Text
            class="text-xs font-semibold text-foreground"
            style={{ fontVariant: ["tabular-nums"] }}
          >
            {activePointIndex() >= 0 ? timelineLabels()[activePointIndex()] : tr("charts.latest")}
          </Text>
          <View class="flex-row flex-wrap items-center gap-3">
            <Each of={visibleSeries()} key={(s) => `legend-${s.profile_id}`}>
              {(s, idx) => {
                const pt = () => {
                  const pts = s().points;
                  const targetIdx = activePointIndex();
                  if (targetIdx < 0 || pts.length === 0) return null;
                  return pts[Math.min(targetIdx, pts.length - 1)] ?? null;
                };
                return (
                  <If when={pt()}>
                    {(p) => (
                      <View class="flex-row items-center gap-1">
                        <Canvas
                          style={{ width: 7, height: 7 }}
                          draw={(cx, size) => {
                            cx.clearRect(0, 0, size.width, size.height);
                            cx.fillStyle = resolveProfileToken(s().profile_id, idx());
                            cx.beginPath();
                            cx.arc(size.width / 2, size.height / 2, 3, 0, Math.PI * 2);
                            cx.fill();
                          }}
                        />
                        <Text class="text-[11px] text-muted-foreground">{s().label}</Text>
                        <Text
                          class="text-xs font-bold text-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {`${Math.round(p().uln_pct)}%`}
                        </Text>
                        <If when={p().raw_value !== undefined && p().raw_value !== null}>
                          <Text
                            class="text-[10px] text-muted-foreground"
                            style={{ fontVariant: ["tabular-nums"] }}
                          >
                            {`(${p().raw_value}${p().unit ? ` ${p().unit}` : ""})`}
                          </Text>
                        </If>
                      </View>
                    )}
                  </If>
                );
              }}
            </Each>
          </View>
        </View>

        <Canvas
          class="w-full"
          style={{ height: 156 }}
          draw={(cx, size) => {
            const w = Math.max(220, size.width);
            const h = size.height;
            const padL = 38;
            const padR = 14;
            const padT = 16;
            const padB = 26;
            const plotW = Math.max(40, w - padL - padR);
            const plotH = Math.max(40, h - padT - padB);

            cx.clearRect(0, 0, w, h);

            const active = visibleSeries();
            const allValues = active.flatMap((s) => s.points.map((p) => p.uln_pct));
            const maxVal = Math.max(135, ...allValues, 100);
            const yMax = Math.ceil(maxVal / 25) * 25;
            const yMin = 0;

            const yFor = (val: number) => {
              const ratio = (val - yMin) / Math.max(1, yMax - yMin);
              return padT + plotH * (1 - Math.max(0, Math.min(1, ratio)));
            };

            const labels = timelineLabels();
            const count = Math.max(1, labels.length);
            const xFor = (i: number) =>
              count === 1 ? padL + plotW / 2 : padL + (plotW * i) / (count - 1);

            // 1) 水平网格线 (50%, 100% ULN, 150%)
            const refLevels = [50, 100, 150].filter((lv) => lv <= yMax);
            for (const lv of refLevels) {
              const y = yFor(lv);
              const isUlnLine = lv === 100;
              cx.save();
              cx.strokeStyle = isUlnLine ? readToken("vital-recover") : readToken("border");
              cx.lineWidth = isUlnLine ? 1.4 : 1;
              cx.globalAlpha = isUlnLine ? 0.78 : 0.65;
              cx.setLineDash(isUlnLine ? [5, 4] : [2, 3]);
              cx.beginPath();
              cx.moveTo(padL, y);
              cx.lineTo(padL + plotW, y);
              cx.stroke();
              cx.restore();

              cx.save();
              cx.font = isUlnLine ? '600 9px "Menlo", monospace' : '500 9px "Menlo", monospace';
              cx.fillStyle = isUlnLine ? readToken("vital-recover") : readToken("ink-subtle");
              cx.textAlign = "right";
              cx.textBaseline = "middle";
              cx.fillText(`${lv}%`, padL - 6, y);
              cx.restore();
            }

            // 100% ULN 标签角标
            const y100 = yFor(100);
            cx.save();
            cx.font = isEnglishLocale()
              ? "600 9px -apple-system, sans-serif"
              : '600 9px "PingFang SC", sans-serif';
            cx.fillStyle = readToken("vital-recover");
            cx.textAlign = "right";
            cx.textBaseline = "bottom";
            cx.fillText(tr("charts.uln_limit"), padL + plotW, y100 - 3);
            cx.restore();

            // 2) 选中时间竖向准星
            const focusIdx = activePointIndex();
            if (focusIdx >= 0 && focusIdx < count) {
              const fx = xFor(focusIdx);
              cx.save();
              cx.strokeStyle = readToken("primary");
              cx.globalAlpha = 0.35;
              cx.lineWidth = 1;
              cx.beginPath();
              cx.moveTo(fx, padT);
              cx.lineTo(fx, padT + plotH);
              cx.stroke();
              cx.restore();
            }

            // 3) 多档案折线与圆点
            active.forEach((series, sIdx) => {
              const strokeCol = resolveProfileToken(series.profile_id, sIdx);
              const pts = series.points;
              if (pts.length === 0) return;

              cx.save();
              cx.strokeStyle = strokeCol;
              cx.lineWidth = 2.2;
              cx.lineJoin = "round";
              cx.lineCap = "round";
              cx.beginPath();
              pts.forEach((pt, idx) => {
                const px = xFor(idx);
                const py = yFor(pt.uln_pct);
                if (idx === 0) cx.moveTo(px, py);
                else cx.lineTo(px, py);
              });
              cx.stroke();
              cx.restore();

              pts.forEach((pt, idx) => {
                const px = xFor(idx);
                const py = yFor(pt.uln_pct);
                const isFocused = idx === focusIdx;
                cx.save();
                cx.fillStyle = readToken("card");
                cx.beginPath();
                cx.arc(px, py, isFocused ? 5.2 : 3.6, 0, Math.PI * 2);
                cx.fill();
                cx.fillStyle = strokeCol;
                cx.beginPath();
                cx.arc(px, py, isFocused ? 3.5 : 2.3, 0, Math.PI * 2);
                cx.fill();
                cx.restore();
              });
            });

            // 4) 底部时间刻度
            cx.save();
            cx.font = '500 9px "Menlo", monospace';
            cx.fillStyle = readToken("ink-subtle");
            cx.textBaseline = "top";
            labels.forEach((lbl, idx) => {
              if (count <= 5 || idx === 0 || idx === count - 1 || idx === Math.floor(count / 2)) {
                cx.textAlign = idx === 0 ? "left" : idx === count - 1 ? "right" : "center";
                cx.fillText(lbl, xFor(idx), padT + plotH + 7);
              }
            });
            cx.restore();
          }}
        />

        <If when={timelineLabels().length > 1}>
          <View class="flex-row flex-wrap items-center gap-1.5">
            <Text class="text-[11px] text-muted-foreground">{tr("charts.inspect_point_label")}</Text>
            <Each of={timelineLabels()} key={(lbl, idx) => `uln-pt-${idx}-${lbl}`}>
              {(lbl, idx) => {
                const isFocused = () => activePointIndex() === idx();
                return (
                  <Button
                    hitSlop={4}
                    onPress={() => selectedIndex.set(idx())}
                    class={`min-h-7 rounded-full border px-2.5 py-0.5 transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-ring motion-reduce:transition-none ${
                      isFocused()
                        ? "border-primary bg-primary"
                        : "border-border/70 bg-surface-sunken hover:border-primary/45"
                    }`}
                  >
                    <Text
                      class={`text-[10px] font-semibold ${
                        isFocused() ? "text-primary-foreground" : "text-muted-foreground"
                      }`}
                      style={{ fontVariant: ["tabular-nums"] }}
                    >
                      {lbl()}
                    </Text>
                  </Button>
                );
              }}
            </Each>
          </View>
        </If>
      </If>
    </View>
  );
}

export interface MultiLagBarItem {
  lag_label: string;
  delta_value: number;
  unit: string;
  cohen_d?: number | null;
  sample_n?: number;
  confounder_note?: string | null;
}

function resolveCohenBadge(d: number | null | undefined): { label: string; isStrong: boolean } {
  if (d === null || d === undefined || !Number.isFinite(d)) {
    return { label: tr("charts.cohen_estimating"), isStrong: false };
  }
  const abs = Math.abs(d);
  const tag =
    abs >= 0.8
      ? tr("charts.cohen_d_large")
      : abs >= 0.5
        ? tr("charts.cohen_d_medium")
        : abs >= 0.2
          ? tr("charts.cohen_d_small")
          : tr("charts.cohen_d_trivial");
  return { label: tr("charts.cohen_d_badge", { d: d.toFixed(2), tag }), isStrong: abs >= 0.5 };
}

/** 90 天多滞后归因横向条（D+0 / D+1 / D+2 效应条 + Cohen's d 徽章 + 混杂变量芯片 + 样本量 n） */
export function MultiLagRadarBars(props: {
  title?: string;
  outcomeLabel?: string;
  items: MultiLagBarItem[];
  sampleDays?: number;
  confounders?: string[];
}) {
  const maxAbs = () => {
    const vals = props.items.map((it) => Math.abs(it.delta_value));
    return Math.max(1, ...vals);
  };

  return (
    <View class="w-full gap-2.5">
      <If when={Boolean(props.title || props.sampleDays)}>
        <View class="flex-row flex-wrap items-center justify-between gap-2">
          <If when={Boolean(props.title)}>
            <Text class="text-xs font-semibold text-foreground">{props.title}</Text>
          </If>
          <If when={props.sampleDays !== undefined}>
            <View class="rounded-full bg-atelier-soft px-2.5 py-0.5">
              <Text
                class="text-[11px] font-semibold text-atelier-accent"
                style={{ fontVariant: ["tabular-nums"] }}
              >
                {tr("charts.window_days", { days: 90, n: props.sampleDays ?? 0 })}
              </Text>
            </View>
          </If>
        </View>
      </If>

      <If when={Boolean(props.confounders && props.confounders.length > 0)}>
        <View class="flex-row flex-wrap items-center gap-1.5">
          <Text class="text-[11px] text-muted-foreground">{tr("charts.confounders_label")}</Text>
          <Each of={props.confounders ?? []} key={(c) => c}>
            {(chip) => (
              <View class="rounded-full border border-border/70 bg-surface-sunken px-2 py-0.5">
                <Text class="text-[10px] font-medium text-foreground">{chip()}</Text>
              </View>
            )}
          </Each>
        </View>
      </If>

      <If
        when={props.items.length > 0}
        else={
          <View class="items-center justify-center rounded-xl border border-border/60 bg-surface-sunken py-5">
            <Text class="text-xs text-muted-foreground">{tr("charts.no_multi_lag_data")}</Text>
          </View>
        }
      >
        <View class="gap-2">
          <Each of={props.items} key={(it) => it.lag_label}>
            {(item) => {
              const cohen = () => resolveCohenBadge(item().cohen_d);
              const signPrefix = () => (item().delta_value >= 0 ? "+" : "");
              return (
                <View class="gap-1.5 rounded-xl border border-border/65 bg-surface-sunken p-3">
                  <View class="flex-row flex-wrap items-center justify-between gap-2">
                    <View class="flex-row items-center gap-2">
                      <View class="rounded-lg border border-border/70 bg-card px-2 py-0.5">
                        <Text
                          class="text-xs font-bold text-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {item().lag_label}
                        </Text>
                      </View>
                      <Text
                        class={`text-xs font-bold ${
                          item().delta_value >= 0 ? "text-vital-prime" : "text-vital-recover"
                        }`}
                        style={{ fontVariant: ["tabular-nums"] }}
                      >
                        {tr("charts.outcome_delta", {
                          label: props.outcomeLabel ?? tr("charts.default_outcome_label"),
                          sign: signPrefix(),
                          value: item().delta_value.toFixed(1),
                          unit: item().unit,
                        })}
                      </Text>
                    </View>

                    <View class="flex-row items-center gap-1.5">
                      <View
                        class={`rounded-full px-2 py-0.5 ${
                          cohen().isStrong ? "bg-atelier-soft" : "bg-card border border-border/60"
                        }`}
                      >
                        <Text
                          class={`text-[10px] font-semibold ${
                            cohen().isStrong ? "text-atelier-accent" : "text-muted-foreground"
                          }`}
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {cohen().label}
                        </Text>
                      </View>
                      <If when={item().sample_n !== undefined}>
                        <Text
                          class="text-[10px] text-muted-foreground"
                          style={{ fontVariant: ["tabular-nums"] }}
                        >
                          {`n=${item().sample_n}`}
                        </Text>
                      </If>
                    </View>
                  </View>

                  <Canvas
                    class="w-full"
                    style={{ height: 14 }}
                    draw={(cx, size) => {
                      const w = Math.max(140, size.width);
                      const h = size.height;
                      const midX = w / 2;
                      const barH = 8;
                      const barY = (h - barH) / 2;

                      cx.clearRect(0, 0, w, h);

                      // 底轨
                      cx.save();
                      cx.fillStyle = readToken("card");
                      cx.strokeStyle = readToken("border");
                      cx.lineWidth = 1;
                      cx.beginPath();
                      cx.roundRect(0, barY, w, barH, barH / 2);
                      cx.fill();
                      cx.stroke();
                      cx.restore();

                      // 中线
                      cx.save();
                      cx.strokeStyle = readToken("muted-foreground");
                      cx.globalAlpha = 0.45;
                      cx.lineWidth = 1;
                      cx.beginPath();
                      cx.moveTo(midX, 1);
                      cx.lineTo(midX, h - 1);
                      cx.stroke();
                      cx.restore();

                      // 双向影响条
                      const ratio = Math.min(1, Math.abs(item().delta_value) / maxAbs());
                      const segW = Math.max(4, (w / 2 - 4) * ratio);
                      const isPositive = item().delta_value >= 0;
                      cx.save();
                      cx.fillStyle = isPositive
                        ? readToken("vital-prime")
                        : readToken("vital-recover");
                      cx.beginPath();
                      if (isPositive) {
                        cx.roundRect(midX + 1, barY + 1, segW, barH - 2, 3);
                      } else {
                        cx.roundRect(midX - 1 - segW, barY + 1, segW, barH - 2, 3);
                      }
                      cx.fill();
                      cx.restore();
                    }}
                  />

                  <If when={Boolean(item().confounder_note)}>
                    <Text class="text-[11px] text-muted-foreground">
                      {tr("charts.confounder_note", { note: item().confounder_note ?? "" })}
                    </Text>
                  </If>
                </View>
              );
            }}
          </Each>
        </View>
      </If>
    </View>
  );
}
