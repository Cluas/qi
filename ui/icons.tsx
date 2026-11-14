import { Canvas, token } from "nsh";

export type QiIconName =
  | "sun"
  | "record"
  | "compass"
  | "user_shield"
  | "user_circle"
  | "sparkles"
  | "check"
  | "plus"
  | "mic"
  | "camera"
  | "coffee"
  | "utensils"
  | "moon"
  | "heart_pulse"
  | "flame"
  | "file_lab"
  | "cloud_lock"
  | "sliders"
  | "chevron_down"
  | "chevron_right"
  | "refresh"
  | "paw"
  | "stamp_seal"
  | "droplet"
  | "tea_cup"
  | "flower_branch"
  | "lantern"
  | "palette"
  | "postcard"
  | "solar_term"
  | "wax_seal"
  | "envelope"
  | "co_sanctuary"
  | "widget"
  | "handscroll"
  | "badge_ceramic"
  | "family_heart"
  | "globe"
  | "custom_pet"
  | "starlight"
  | "hearth_ember"
  | "restore_arrow"
  | "motion_toggle";

/** Retina 2D 矢量线性图标（零外部字体依赖，动态读取主题 Token） */
export function QiIcon(props: {
  name: QiIconName;
  size?: number;
  color?: string;
  strokeWidth?: number;
}) {
  return (
    <Canvas
      style={{
        width: props.size ?? 20,
        height: props.size ?? 20,
      }}
      draw={(cx, s) => {
        const strokeColor = props.color ?? token("foreground");
        const sw = props.strokeWidth ?? 1.85;

        cx.clearRect(0, 0, s.width, s.height);
        cx.save();
        cx.scale(s.width / 24, s.height / 24);
        cx.strokeStyle = strokeColor;
        cx.fillStyle = strokeColor;
        cx.lineWidth = sw;
        cx.lineCap = "round";
        cx.lineJoin = "round";

        switch (props.name) {
          case "sun": {
            cx.beginPath();
            cx.arc(12, 12, 4.2, 0, Math.PI * 2);
            cx.stroke();
            for (let i = 0; i < 8; i++) {
              const a = (i * Math.PI) / 4;
              cx.beginPath();
              cx.moveTo(12 + Math.cos(a) * 7.2, 12 + Math.sin(a) * 7.2);
              cx.lineTo(12 + Math.cos(a) * 9.8, 12 + Math.sin(a) * 9.8);
              cx.stroke();
            }
            break;
          }

          case "record": {
            cx.beginPath();
            cx.roundRect(5, 3.5, 14, 17, 2.5);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(8.5, 8.5);
            cx.lineTo(15.5, 8.5);
            cx.moveTo(8.5, 12.5);
            cx.lineTo(15.5, 12.5);
            cx.moveTo(8.5, 16.5);
            cx.lineTo(13, 16.5);
            cx.stroke();
            break;
          }

          case "compass": {
            cx.beginPath();
            cx.arc(12, 12, 9, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(15.6, 8.4);
            cx.lineTo(13.4, 13.4);
            cx.lineTo(8.4, 15.6);
            cx.lineTo(10.6, 10.6);
            cx.closePath();
            cx.stroke();
            break;
          }

          case "user_shield": {
            cx.beginPath();
            cx.moveTo(12, 2.8);
            cx.lineTo(19, 5.8);
            cx.lineTo(19, 11.2);
            cx.bezierCurveTo(19, 15.8, 15.8, 19.3, 12, 21);
            cx.bezierCurveTo(8.2, 19.3, 5, 15.8, 5, 11.2);
            cx.lineTo(5, 5.8);
            cx.closePath();
            cx.stroke();

            cx.beginPath();
            cx.arc(12, 9.6, 2.1, 0, Math.PI * 2);
            cx.stroke();

            cx.beginPath();
            cx.arc(12, 16.5, 3.8, Math.PI * 1.18, Math.PI * 1.82);
            cx.stroke();
            break;
          }

          case "user_circle": {
            cx.beginPath();
            cx.arc(12, 12, 8.8, 0, Math.PI * 2);
            cx.stroke();

            cx.beginPath();
            cx.arc(12, 9.4, 2.5, 0, Math.PI * 2);
            cx.stroke();

            cx.beginPath();
            cx.arc(12, 19.6, 5.2, Math.PI * 1.18, Math.PI * 1.82);
            cx.stroke();
            break;
          }

          case "sparkles": {
            cx.beginPath();
            cx.moveTo(10.5, 5.2);
            cx.quadraticCurveTo(11.3, 11.7, 17.8, 12.5);
            cx.quadraticCurveTo(11.3, 13.3, 10.5, 19.8);
            cx.quadraticCurveTo(9.7, 13.3, 3.2, 12.5);
            cx.quadraticCurveTo(9.7, 11.7, 10.5, 5.2);
            cx.closePath();
            cx.stroke();

            cx.beginPath();
            cx.moveTo(18.2, 3.5);
            cx.lineTo(18.2, 7.9);
            cx.moveTo(16, 5.7);
            cx.lineTo(20.4, 5.7);
            cx.stroke();
            break;
          }

          case "check": {
            cx.beginPath();
            cx.moveTo(5, 12.5);
            cx.lineTo(9.5, 17);
            cx.lineTo(19, 7.5);
            cx.stroke();
            break;
          }

          case "plus": {
            cx.beginPath();
            cx.moveTo(12, 5);
            cx.lineTo(12, 19);
            cx.moveTo(5, 12);
            cx.lineTo(19, 12);
            cx.stroke();
            break;
          }

          case "mic": {
            cx.beginPath();
            cx.roundRect(9, 3, 6, 10, 3);
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 10.5, 5.8, Math.PI * 0.08, Math.PI * 0.92);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(12, 16.3);
            cx.lineTo(12, 20.5);
            cx.moveTo(8.5, 20.5);
            cx.lineTo(15.5, 20.5);
            cx.stroke();
            break;
          }

          case "camera": {
            cx.beginPath();
            cx.roundRect(3, 6.5, 18, 13.5, 2.5);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(8.2, 6.5);
            cx.lineTo(9.8, 4);
            cx.lineTo(14.2, 4);
            cx.lineTo(15.8, 6.5);
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 13.2, 3.2, 0, Math.PI * 2);
            cx.stroke();
            break;
          }

          case "coffee": {
            cx.beginPath();
            cx.moveTo(4, 9);
            cx.lineTo(16.5, 9);
            cx.lineTo(16.5, 14.5);
            cx.bezierCurveTo(16.5, 17.8, 13.8, 20, 10.25, 20);
            cx.bezierCurveTo(6.7, 20, 4, 17.8, 4, 14.5);
            cx.closePath();
            cx.stroke();

            cx.beginPath();
            cx.moveTo(16.5, 10.5);
            cx.lineTo(18.5, 10.5);
            cx.arc(18.5, 12.75, 2.25, -Math.PI / 2, Math.PI / 2);
            cx.lineTo(16.5, 15);
            cx.stroke();

            cx.beginPath();
            cx.moveTo(8, 6.2);
            cx.quadraticCurveTo(8.8, 4.9, 8, 3.5);
            cx.moveTo(12, 6.2);
            cx.quadraticCurveTo(12.8, 4.9, 12, 3.5);
            cx.stroke();
            break;
          }

          case "utensils": {
            cx.beginPath();
            cx.moveTo(6, 4);
            cx.lineTo(6, 9.5);
            cx.quadraticCurveTo(8, 12.2, 10, 9.5);
            cx.lineTo(10, 4);
            cx.moveTo(8, 4);
            cx.lineTo(8, 20);
            cx.stroke();

            cx.beginPath();
            cx.moveTo(16.2, 20);
            cx.lineTo(16.2, 4);
            cx.bezierCurveTo(19.5, 5.5, 19.5, 11, 16.2, 13);
            cx.stroke();
            break;
          }

          case "moon": {
            cx.beginPath();
            cx.arc(12, 12, 8.5, -Math.PI * 0.55, Math.PI * 0.25, true);
            cx.quadraticCurveTo(9.5, 14.2, 10.67, 3.6);
            cx.closePath();
            cx.stroke();
            break;
          }

          case "heart_pulse": {
            cx.beginPath();
            cx.moveTo(12, 20.2);
            cx.bezierCurveTo(5.5, 15.8, 3, 12.2, 3, 8.3);
            cx.bezierCurveTo(3, 5.4, 5.3, 3.5, 7.9, 3.5);
            cx.bezierCurveTo(9.8, 3.5, 11.3, 4.5, 12, 5.8);
            cx.bezierCurveTo(12.7, 4.5, 14.2, 3.5, 16.1, 3.5);
            cx.bezierCurveTo(18.7, 3.5, 21, 5.4, 21, 8.3);
            cx.bezierCurveTo(21, 12.2, 18.5, 15.8, 12, 20.2);
            cx.closePath();
            cx.stroke();

            cx.beginPath();
            cx.moveTo(6.5, 11.5);
            cx.lineTo(9.1, 11.5);
            cx.lineTo(10.8, 8.6);
            cx.lineTo(13, 14.2);
            cx.lineTo(14.7, 11.5);
            cx.lineTo(17.5, 11.5);
            cx.stroke();
            break;
          }

          case "flame": {
            cx.beginPath();
            cx.moveTo(12, 2.8);
            cx.bezierCurveTo(14.8, 6.2, 18.4, 9.2, 18.4, 14);
            cx.arc(12, 14, 6.4, 0, Math.PI, false);
            cx.bezierCurveTo(5.6, 10.4, 8, 7.6, 9.2, 6.2);
            cx.bezierCurveTo(9.5, 8.2, 10.9, 9.2, 11, 9.2);
            cx.bezierCurveTo(10.9, 6.8, 12, 2.8, 12, 2.8);
            cx.closePath();
            cx.stroke();
            break;
          }

          case "file_lab": {
            cx.beginPath();
            cx.roundRect(5, 3, 14, 18, 2.2);
            cx.stroke();

            cx.beginPath();
            cx.moveTo(10.2, 7.5);
            cx.lineTo(13.8, 7.5);
            cx.moveTo(11, 7.5);
            cx.lineTo(11, 10.4);
            cx.lineTo(8.5, 15.2);
            cx.lineTo(15.5, 15.2);
            cx.lineTo(13, 10.4);
            cx.lineTo(13, 7.5);
            cx.moveTo(8.8, 17.8);
            cx.lineTo(15.2, 17.8);
            cx.stroke();
            break;
          }

          case "cloud_lock": {
            cx.beginPath();
            cx.moveTo(6.5, 13.5);
            cx.bezierCurveTo(4.2, 13.5, 3, 11.8, 3.5, 9.8);
            cx.bezierCurveTo(3.9, 8.1, 5.4, 7.2, 7, 7.3);
            cx.bezierCurveTo(7.8, 4.7, 10.3, 3.4, 12.8, 3.9);
            cx.bezierCurveTo(15.2, 4.4, 16.9, 6.3, 17.1, 8.6);
            cx.bezierCurveTo(19.2, 8.7, 20.8, 10.2, 20.6, 12.2);
            cx.bezierCurveTo(20.5, 13, 20, 13.5, 19.3, 13.8);
            cx.stroke();

            cx.beginPath();
            cx.roundRect(8.5, 14, 7, 6, 1.5);
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 14, 2.1, Math.PI, 0);
            cx.stroke();
            break;
          }

          case "sliders": {
            cx.beginPath();
            cx.moveTo(4, 6.5);
            cx.lineTo(7.5, 6.5);
            cx.moveTo(11.5, 6.5);
            cx.lineTo(20, 6.5);
            cx.moveTo(4, 12);
            cx.lineTo(13.5, 12);
            cx.moveTo(17.5, 12);
            cx.lineTo(20, 12);
            cx.moveTo(4, 17.5);
            cx.lineTo(8.5, 17.5);
            cx.moveTo(12.5, 17.5);
            cx.lineTo(20, 17.5);
            cx.stroke();

            cx.beginPath();
            cx.arc(9.5, 6.5, 2, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.arc(15.5, 12, 2, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.arc(10.5, 17.5, 2, 0, Math.PI * 2);
            cx.stroke();
            break;
          }

          case "chevron_down": {
            cx.beginPath();
            cx.moveTo(6.5, 9.5);
            cx.lineTo(12, 15);
            cx.lineTo(17.5, 9.5);
            cx.stroke();
            break;
          }

          case "chevron_right": {
            cx.beginPath();
            cx.moveTo(9.5, 6.5);
            cx.lineTo(15, 12);
            cx.lineTo(9.5, 17.5);
            cx.stroke();
            break;
          }

          case "refresh": {
            cx.beginPath();
            cx.arc(12, 12, 7.5, Math.PI * 0.16, Math.PI * 1.82, false);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(15.2, 7.9);
            cx.lineTo(18.5, 7.9);
            cx.lineTo(18.5, 4.6);
            cx.stroke();
            break;
          }

          case "paw": {
            // 猫爪主肉垫 + 四枚小趾垫
            cx.beginPath();
            cx.moveTo(8, 16.5);
            cx.bezierCurveTo(7.4, 14.2, 9.3, 12.2, 12, 12.2);
            cx.bezierCurveTo(14.7, 12.2, 16.6, 14.2, 16, 16.5);
            cx.bezierCurveTo(15.5, 18.6, 13.7, 19.3, 12, 19);
            cx.bezierCurveTo(10.3, 19.3, 8.5, 18.6, 8, 16.5);
            cx.closePath();
            cx.stroke();

            const toes: [number, number][] = [
              [6.4, 10.6],
              [9.9, 8],
              [14.1, 8],
              [17.6, 10.6],
            ];
            for (const [tx, ty] of toes) {
              cx.beginPath();
              cx.arc(tx, ty, 1.55, 0, Math.PI * 2);
              cx.stroke();
            }
            break;
          }

          case "stamp_seal": {
            // 朱砂方印（外框 + 篆意十字曲纹）
            cx.beginPath();
            cx.roundRect(4.5, 4.5, 15, 15, 2.2);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(9, 8.5);
            cx.lineTo(15, 8.5);
            cx.moveTo(12, 8.5);
            cx.lineTo(12, 15.5);
            cx.moveTo(8.8, 12);
            cx.lineTo(15.2, 12);
            cx.moveTo(8.8, 15.5);
            cx.lineTo(15.2, 15.5);
            cx.stroke();
            break;
          }

          case "droplet": {
            // 晨露水滴
            cx.beginPath();
            cx.moveTo(12, 3.6);
            cx.bezierCurveTo(15.5, 8.5, 18.2, 11.4, 18.2, 14.6);
            cx.arc(12, 14.6, 6.2, 0, Math.PI, false);
            cx.bezierCurveTo(5.8, 11.4, 8.5, 8.5, 12, 3.6);
            cx.closePath();
            cx.stroke();
            break;
          }

          case "tea_cup": {
            // 青瓷温茶盏
            cx.beginPath();
            cx.moveTo(5.2, 10);
            cx.lineTo(18.8, 10);
            cx.bezierCurveTo(18.6, 15.6, 15.6, 18.8, 12, 18.8);
            cx.bezierCurveTo(8.4, 18.8, 5.4, 15.6, 5.2, 10);
            cx.closePath();
            cx.stroke();

            cx.beginPath();
            cx.moveTo(8.5, 20);
            cx.lineTo(15.5, 20);
            cx.moveTo(9.4, 6.8);
            cx.quadraticCurveTo(10.3, 5.2, 9.4, 3.8);
            cx.moveTo(14.2, 6.8);
            cx.quadraticCurveTo(15.1, 5.2, 14.2, 3.8);
            cx.stroke();
            break;
          }

          case "flower_branch": {
            // 金桂/松枝清供
            cx.beginPath();
            cx.moveTo(6.5, 19.5);
            cx.quadraticCurveTo(11, 13.5, 16.5, 5.5);
            cx.stroke();

            const blossoms: [number, number][] = [
              [10, 12],
              [13.6, 9.5],
              [16.2, 12.2],
              [11.2, 7.8],
            ];
            for (const [bx, by] of blossoms) {
              cx.beginPath();
              cx.arc(bx, by, 1.7, 0, Math.PI * 2);
              cx.stroke();
            }
            break;
          }

          case "lantern": {
            // 暖纸小夜灯
            cx.beginPath();
            cx.moveTo(12, 3);
            cx.lineTo(12, 5.5);
            cx.stroke();
            cx.beginPath();
            cx.roundRect(6.5, 5.5, 11, 12, 4);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(8.5, 17.5);
            cx.lineTo(15.5, 17.5);
            cx.moveTo(10, 20.2);
            cx.lineTo(14, 20.2);
            cx.moveTo(12, 9.5);
            cx.lineTo(12, 13.5);
            cx.stroke();
            break;
          }

          case "palette": {
            // 灵境画风调色盘
            cx.beginPath();
            cx.arc(12, 12, 8.6, 0, Math.PI * 1.62, false);
            cx.bezierCurveTo(15.8, 12, 14.5, 15.2, 16.8, 16.2);
            cx.stroke();
            for (const [px, py] of [
              [8.5, 10],
              [11.5, 7.6],
              [15.4, 9.2],
              [9, 14.2],
            ] as [number, number][]) {
              cx.beginPath();
              cx.arc(px, py, 1.15, 0, Math.PI * 2);
              cx.fill();
            }
            break;
          }

          case "postcard": {
            // 今日日签明信片
            cx.beginPath();
            cx.roundRect(3.5, 5, 17, 14, 2.2);
            cx.stroke();
            cx.beginPath();
            cx.roundRect(14, 7.8, 3.8, 4.2, 0.8);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(6.5, 14.5);
            cx.lineTo(11.5, 14.5);
            cx.moveTo(6.5, 11.2);
            cx.lineTo(10.5, 11.2);
            cx.stroke();
            break;
          }

          case "solar_term": {
            // 二十四节气日轮与嫩芽
            cx.beginPath();
            cx.arc(12, 12, 8.2, 0, Math.PI * 2);
            cx.stroke();
            for (let i = 0; i < 4; i++) {
              const a = (i * Math.PI) / 2;
              cx.beginPath();
              cx.moveTo(12 + Math.cos(a) * 8.2, 12 + Math.sin(a) * 8.2);
              cx.lineTo(12 + Math.cos(a) * 10.4, 12 + Math.sin(a) * 10.4);
              cx.stroke();
            }
            cx.beginPath();
            cx.moveTo(12, 16.5);
            cx.lineTo(12, 9.2);
            cx.moveTo(12, 13);
            cx.quadraticCurveTo(15.6, 12.6, 15.8, 9.6);
            cx.quadraticCurveTo(12.6, 9.6, 12, 13);
            cx.stroke();
            break;
          }

          case "wax_seal": {
            // 火漆封蜡印记
            cx.beginPath();
            cx.arc(12, 12, 8.2, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 12, 5.2, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(10.2, 10.6);
            cx.lineTo(13.8, 10.6);
            cx.moveTo(12, 9.4);
            cx.lineTo(12, 14.6);
            cx.moveTo(10.2, 13.8);
            cx.lineTo(13.8, 13.8);
            cx.stroke();
            break;
          }

          case "envelope": {
            // 周日慢递信笺
            cx.beginPath();
            cx.roundRect(3.2, 5.5, 17.6, 13, 2.2);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(4.2, 6.8);
            cx.lineTo(12, 12.6);
            cx.lineTo(19.8, 6.8);
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 13.2, 1.3, 0, Math.PI * 2);
            cx.fill();
            break;
          }

          case "co_sanctuary": {
            // 同窗共养 · 双枝暖窗
            cx.beginPath();
            cx.roundRect(4, 4, 16, 15, 2.4);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(3, 19);
            cx.lineTo(21, 19);
            cx.moveTo(12, 4);
            cx.lineTo(12, 19);
            cx.moveTo(8, 15.5);
            cx.quadraticCurveTo(7.4, 11.8, 9.8, 10.4);
            cx.moveTo(16, 15.5);
            cx.quadraticCurveTo(16.6, 11.8, 14.2, 10.4);
            cx.stroke();
            break;
          }

          case "widget": {
            // 抬腕微件 · 表盘弧
            cx.beginPath();
            cx.roundRect(5.5, 5.5, 13, 13, 3.6);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(9, 5.5);
            cx.lineTo(9.5, 3.2);
            cx.lineTo(14.5, 3.2);
            cx.lineTo(15, 5.5);
            cx.moveTo(9, 18.5);
            cx.lineTo(9.5, 20.8);
            cx.lineTo(14.5, 20.8);
            cx.lineTo(15, 18.5);
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 12.2, 3.6, Math.PI * 0.85, Math.PI * 2.15);
            cx.stroke();
            break;
          }

          case "handscroll": {
            // 四时风物长卷
            cx.beginPath();
            cx.roundRect(3, 6.2, 2.8, 11.6, 1.4);
            cx.roundRect(18.2, 6.2, 2.8, 11.6, 1.4);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(5.8, 7.2);
            cx.lineTo(18.2, 7.2);
            cx.moveTo(5.8, 16.8);
            cx.lineTo(18.2, 16.8);
            cx.moveTo(7.4, 13.6);
            cx.quadraticCurveTo(10.2, 9.6, 12.4, 12.2);
            cx.quadraticCurveTo(14.6, 14.6, 16.8, 10.6);
            cx.stroke();
            break;
          }

          case "badge_ceramic": {
            // 宋瓷花口勋章
            cx.beginPath();
            cx.moveTo(12, 3.4);
            cx.lineTo(17.8, 6.6);
            cx.lineTo(19.2, 12.6);
            cx.lineTo(15.6, 17.8);
            cx.lineTo(12, 20.6);
            cx.lineTo(8.4, 17.8);
            cx.lineTo(4.8, 12.6);
            cx.lineTo(6.2, 6.6);
            cx.closePath();
            cx.stroke();
            cx.beginPath();
            cx.arc(12, 11.8, 3.2, 0, Math.PI * 2);
            cx.stroke();
            break;
          }

          case "family_heart": {
            // 气韵家书暖心
            cx.beginPath();
            cx.moveTo(12, 20);
            cx.bezierCurveTo(5.6, 15.6, 3.2, 12, 3.2, 8.4);
            cx.bezierCurveTo(3.2, 5.6, 5.4, 3.8, 8, 3.8);
            cx.bezierCurveTo(9.8, 3.8, 11.2, 4.8, 12, 6.1);
            cx.bezierCurveTo(12.8, 4.8, 14.2, 3.8, 16, 3.8);
            cx.bezierCurveTo(18.6, 3.8, 20.8, 5.6, 20.8, 8.4);
            cx.bezierCurveTo(20.8, 12, 18.4, 15.6, 12, 20);
            cx.closePath();
            cx.stroke();
            cx.beginPath();
            cx.moveTo(8.5, 11.8);
            cx.lineTo(12, 9);
            cx.lineTo(15.5, 11.8);
            cx.moveTo(10, 14.5);
            cx.lineTo(14, 14.5);
            cx.stroke();
            break;
          }
          case "globe": {
            // 界面语言地球仪
            cx.beginPath();
            cx.arc(12, 12, 8.8, 0, Math.PI * 2);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(3.2, 12);
            cx.lineTo(20.8, 12);
            cx.moveTo(5.2, 7.6);
            cx.bezierCurveTo(9, 8.8, 15, 8.8, 18.8, 7.6);
            cx.moveTo(5.2, 16.4);
            cx.bezierCurveTo(9, 15.2, 15, 15.2, 18.8, 16.4);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(12, 3.2);
            cx.bezierCurveTo(8.3, 7, 8.3, 17, 12, 20.8);
            cx.moveTo(12, 3.2);
            cx.bezierCurveTo(15.7, 7, 15.7, 17, 12, 20.8);
            cx.stroke();
            break;
          }
          case "custom_pet": {
            // 自定陪伴工坊：窗台猫耳剪影与印记点
            cx.beginPath();
            cx.moveTo(5.4, 19.2);
            cx.lineTo(5.4, 9.8);
            cx.lineTo(6.6, 5.2);
            cx.lineTo(10.1, 8.1);
            cx.bezierCurveTo(11.3, 7.7, 12.7, 7.7, 13.9, 8.1);
            cx.lineTo(17.4, 5.2);
            cx.lineTo(18.6, 9.8);
            cx.lineTo(18.6, 19.2);
            cx.closePath();
            cx.stroke();
            cx.beginPath();
            cx.arc(9.8, 12.6, 0.9, 0, Math.PI * 2);
            cx.arc(14.2, 12.6, 0.9, 0, Math.PI * 2);
            cx.fill();
            cx.beginPath();
            cx.moveTo(10.8, 15.2);
            cx.bezierCurveTo(11.4, 16.1, 12.6, 16.1, 13.2, 15.2);
            cx.stroke();
            break;
          }
          case "starlight": {
            // 星光纪念守候星芒
            cx.beginPath();
            cx.moveTo(12, 3.4);
            cx.bezierCurveTo(12.8, 9.1, 14.9, 11.2, 20.6, 12);
            cx.bezierCurveTo(14.9, 12.8, 12.8, 14.9, 12, 20.6);
            cx.bezierCurveTo(11.2, 14.9, 9.1, 12.8, 3.4, 12);
            cx.bezierCurveTo(9.1, 11.2, 11.2, 9.1, 12, 3.4);
            cx.closePath();
            cx.stroke();
            cx.beginPath();
            cx.arc(18.2, 6, 1.1, 0, Math.PI * 2);
            cx.arc(6.2, 17.8, 0.85, 0, Math.PI * 2);
            cx.fill();
            break;
          }
          case "hearth_ember": {
            // 窗台添柴微火
            cx.beginPath();
            cx.moveTo(12, 3.6);
            cx.bezierCurveTo(16.6, 7.4, 18.2, 11.3, 16.8, 14.8);
            cx.bezierCurveTo(15.8, 17, 14, 17.8, 12, 17.8);
            cx.bezierCurveTo(9.8, 17.8, 7.8, 16.6, 7.2, 14.3);
            cx.bezierCurveTo(6.4, 11.2, 8.6, 8, 12, 3.6);
            cx.closePath();
            cx.stroke();
            cx.beginPath();
            cx.moveTo(12, 10.2);
            cx.bezierCurveTo(13.6, 12.1, 13.8, 14.2, 12, 15.4);
            cx.bezierCurveTo(10.2, 14.2, 10.4, 12.1, 12, 10.2);
            cx.closePath();
            cx.fill();
            cx.beginPath();
            cx.moveTo(5.8, 20.4);
            cx.lineTo(18.2, 20.4);
            cx.stroke();
            break;
          }
          case "restore_arrow": {
            // App Store 恢复购买回溯环
            cx.beginPath();
            cx.arc(12.2, 12, 7.5, -Math.PI * 0.72, Math.PI * 0.88);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(6.4, 4.9);
            cx.lineTo(6.1, 9);
            cx.lineTo(10.2, 8.6);
            cx.stroke();
            cx.beginPath();
            cx.moveTo(9.4, 12.3);
            cx.lineTo(11.3, 14.2);
            cx.lineTo(15.2, 10.1);
            cx.stroke();
            break;
          }
          case "motion_toggle": {
            // 动效韵律双波纹
            cx.beginPath();
            cx.moveTo(3.5, 9.5);
            cx.bezierCurveTo(6.8, 5.8, 10.2, 5.8, 13.4, 9.5);
            cx.bezierCurveTo(16.1, 12.6, 18.6, 12.6, 20.6, 9.5);
            cx.moveTo(3.5, 15.4);
            cx.bezierCurveTo(6.8, 11.8, 10.2, 11.8, 13.4, 15.4);
            cx.bezierCurveTo(16.1, 18.4, 18.6, 18.4, 20.6, 15.4);
            cx.stroke();
            break;
          }
        }

        cx.restore();
      }}
    />
  );
}

export interface QiIconHelperProps {
  size?: number;
  color?: string;
  strokeWidth?: number;
}

export function SolarTermIcon(props: QiIconHelperProps) {
  return <QiIcon name="solar_term" {...props} />;
}

export function WaxSealIcon(props: QiIconHelperProps) {
  return <QiIcon name="wax_seal" {...props} />;
}

export function EnvelopeIcon(props: QiIconHelperProps) {
  return <QiIcon name="envelope" {...props} />;
}

export function CoSanctuaryIcon(props: QiIconHelperProps) {
  return <QiIcon name="co_sanctuary" {...props} />;
}

export function WidgetIcon(props: QiIconHelperProps) {
  return <QiIcon name="widget" {...props} />;
}

export function HandscrollIcon(props: QiIconHelperProps) {
  return <QiIcon name="handscroll" {...props} />;
}

export function BadgeCeramicIcon(props: QiIconHelperProps) {
  return <QiIcon name="badge_ceramic" {...props} />;
}

export function FamilyHeartIcon(props: QiIconHelperProps) {
  return <QiIcon name="family_heart" {...props} />;
}

export function GlobeIcon(props: QiIconHelperProps) {
  return <QiIcon name="globe" {...props} />;
}

export function CustomPetIcon(props: QiIconHelperProps) {
  return <QiIcon name="custom_pet" {...props} />;
}

export function StarlightIcon(props: QiIconHelperProps) {
  return <QiIcon name="starlight" {...props} />;
}

export function HearthEmberIcon(props: QiIconHelperProps) {
  return <QiIcon name="hearth_ember" {...props} />;
}

export function RestoreArrowIcon(props: QiIconHelperProps) {
  return <QiIcon name="restore_arrow" {...props} />;
}

export function MotionToggleIcon(props: QiIconHelperProps) {
  return <QiIcon name="motion_toggle" {...props} />;
}
