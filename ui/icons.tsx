import { Canvas, token } from "nsh";

export type QiIconName =
  | "sun"
  | "record"
  | "compass"
  | "user_shield"
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
  | "refresh";

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
        }

        cx.restore();
      }}
    />
  );
}
