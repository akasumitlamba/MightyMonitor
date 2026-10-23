/* Original MightyMonitor screen playground. Three.js is vendored under MIT. */
"use strict";
(() => {
  const mount = document.querySelector(".playground");
  if (!mount || !window.THREE) return;
  const T = window.THREE;
  let renderer;
  try {
    renderer = new T.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: "low-power",
    });
  } catch {
    return;
  } // The complete CSS illustration remains available.
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.5));
  renderer.outputColorSpace = T.SRGBColorSpace;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = T.PCFSoftShadowMap;
  renderer.shadowMap.autoUpdate = false;
  renderer.shadowMap.needsUpdate = true;
  renderer.setClearColor(0x000000, 0);
  renderer.domElement.setAttribute("aria-hidden", "true");
  mount.append(renderer.domElement);
  mount.classList.add("rendered");
  const scene = new T.Scene();
  const camera = new T.PerspectiveCamera(30, 1, 0.1, 80);
  camera.position.set(0, 4.6, 12.2);
  camera.lookAt(0, 1.4, 0);
  scene.add(new T.HemisphereLight(0xffffff, 0x869182, 3));
  const key = new T.DirectionalLight(0xffffff, 4);
  key.position.set(-4, 9, 6);
  key.castShadow = true;
  key.shadow.mapSize.set(2048, 2048);
  Object.assign(key.shadow.camera, {
    left: -9,
    right: 9,
    top: 7,
    bottom: -7,
    near: 0.1,
    far: 30,
  });
  key.shadow.bias = -0.0005;
  key.shadow.normalBias = 0.03;
  key.shadow.radius = 4;
  scene.add(key);
  const fill = new T.DirectionalLight(0xecffd9, 1.4);
  fill.position.set(7, 4, -4);
  scene.add(fill);
  const floor = new T.Mesh(
    new T.PlaneGeometry(80, 80),
    new T.ShadowMaterial({ opacity: 0.15 }),
  );
  floor.rotation.x = -Math.PI / 2;
  floor.position.y = -0.05;
  floor.receiveShadow = true;
  scene.add(floor);
  const desk = new T.Group();
  scene.add(desk);
  const graphite = new T.MeshStandardMaterial({
    color: 0x29372e,
    roughness: 0.3,
    metalness: 0.35,
  });
  const silver = new T.MeshStandardMaterial({
    color: 0xcdd3cb,
    roughness: 0.35,
    metalness: 0.65,
  });
  const leaf = new T.MeshStandardMaterial({ color: 0xb9e66b, roughness: 0.6 });
  function shape(w, h, r) {
    const s = new T.Shape(),
      x = -w / 2,
      y = -h / 2;
    s.moveTo(x + r, y);
    s.lineTo(x + w - r, y);
    s.quadraticCurveTo(x + w, y, x + w, y + r);
    s.lineTo(x + w, y + h - r);
    s.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    s.lineTo(x + r, y + h);
    s.quadraticCurveTo(x, y + h, x, y + h - r);
    s.lineTo(x, y + r);
    s.quadraticCurveTo(x, y, x + r, y);
    return s;
  }
  function screenGeometry(w, h, r) {
    const g = new T.ShapeGeometry(shape(w, h, r));
    const p = g.attributes.position,
      u = g.attributes.uv;
    for (let i = 0; i < p.count; i++)
      u.setXY(i, (p.getX(i) + w / 2) / w, (p.getY(i) + h / 2) / h);
    return g;
  }
  function slab(w, h, d, r, material) {
    const mesh = new T.Mesh(
      new T.ExtrudeGeometry(shape(w, h, r), {
        depth: d,
        bevelEnabled: true,
        bevelSegments: 3,
        steps: 1,
        bevelSize: 0.025,
        bevelThickness: 0.025,
        curveSegments: 10,
      }),
      material,
    );
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    return mesh;
  }
  function box(w, h, d, mat, x, y, z) {
    const m = new T.Mesh(new T.BoxGeometry(w, h, d), mat);
    m.position.set(x, y, z);
    m.castShadow = true;
    m.receiveShadow = true;
    return m;
  }
  const devices = [];
  function device(w, h, x, y, z, angle, kind) {
    const group = new T.Group();
    group.position.set(x, y, z);
    group.rotation.y = angle;
    desk.add(group);
    group.add(
      slab(
        w,
        h,
        0.15,
        kind === "phone" ? 0.21 : 0.12,
        kind === "laptop" ? silver : graphite,
      ),
    );
    const canvas = document.createElement("canvas");
    canvas.width = kind === "phone" ? 384 : 640;
    canvas.height = Math.round((canvas.width * h) / w);
    const texture = new T.CanvasTexture(canvas);
    texture.colorSpace = T.SRGBColorSpace;
    const screen = new T.Mesh(
      screenGeometry(w - 0.16, h - 0.16, kind === "phone" ? 0.16 : 0.06),
      new T.MeshBasicMaterial({ map: texture }),
    );
    screen.position.z = 0.18;
    group.add(screen);
    if (kind === "monitor") {
      group.add(box(0.38, y - 0.1, 0.24, silver, 0, -y / 2 - 0.02, -0.08));
      group.add(box(1.65, 0.08, 0.85, silver, 0, -y + 0.03, 0.1));
    } else if (kind === "laptop") {
      const base = slab(w + 0.3, 2.05, 0.09, 0.13, silver);
      base.rotation.x = -Math.PI / 2;
      base.position.set(0, -h / 2, 0.95);
      group.add(base);
      const keys = new T.InstancedMesh(
        new T.BoxGeometry(0.23, 0.018, 0.12),
        graphite,
        40,
      );
      const keyPose = new T.Object3D();
      for (let row = 0; row < 4; row++)
        for (let col = 0; col < 10; col++) {
          keyPose.position.set(
            -1.35 + col * 0.3,
            -h / 2 + 0.11,
            0.29 + row * 0.18,
          );
          keyPose.updateMatrix();
          keys.setMatrixAt(row * 10 + col, keyPose.matrix);
        }
      group.add(keys);
      const pad = box(
        0.95,
        0.013,
        0.47,
        new T.MeshStandardMaterial({
          color: 0xaebaae,
          metalness: 0.5,
          roughness: 0.5,
        }),
        0,
        -h / 2 + 0.11,
        1.4,
      );
      group.add(pad);
    } else {
      const cameraDot = new T.Mesh(new T.CircleGeometry(0.035, 20), graphite);
      cameraDot.position.set(0, h / 2 - 0.13, 0.185);
      group.add(cameraDot);
    }
    devices.push({
      group,
      canvas,
      texture,
      kind,
      base: new T.Vector3(x, y, z),
    });
  }
  device(4.55, 2.85, -2.35, 2.05, -0.15, 0.24, "monitor");
  device(3.65, 2.35, 2.3, 1.65, 0.45, -0.35, "laptop");
  device(1.23, 2.5, 0.18, 1.3, 2.2, -0.16, "phone");
  // A physical connecting ribbon, deliberately static: screen content supplies motion.
  const curve = new T.CatmullRomCurve3([
    new T.Vector3(-2.1, 0.04, 0.2),
    new T.Vector3(-1, 0.12, 2.2),
    new T.Vector3(1, 0.08, 2.8),
    new T.Vector3(2.7, 0.07, 1.3),
  ]);
  const ribbon = new T.Mesh(
    new T.TubeGeometry(curve, 70, 0.045, 8, false),
    leaf,
  );
  ribbon.castShadow = true;
  desk.add(ribbon);
  let mode = "mirror",
    clock = 0,
    last = 0,
    frame = 0,
    visible = true,
    paused = false,
    targetX = 0,
    targetY = 0,
    layout = 0,
    lost = false;
  const reduced = matchMedia("(prefers-reduced-motion: reduce)");
  const rearrange = document.querySelector(".rearrange");
  function rounded(ctx, x, y, w, h, r, fill) {
    ctx.fillStyle = fill;
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
    ctx.fill();
  }
  function paint(d, index) {
    // The phone UI is static within a mode. Do not upload its tall texture on
    // every video frame; only the desktop windows need continuous repainting.
    if (d.kind === "phone" && d.paintedMode === mode) return;
    d.paintedMode = mode;
    const c = d.canvas,
      ctx = c.getContext("2d"),
      w = c.width,
      h = c.height;
    const scale = w / 960;
    ctx.setTransform(scale, 0, 0, scale, 0, 0);
    const H = h / scale;
    ctx.fillStyle = "#e8efdd";
    ctx.fillRect(0, 0, 960, H);
    const offset = mode === "extend" ? index * 260 : 0;
    // One authored landscape, reframed across screens rather than unrelated mock UI.
    ctx.fillStyle = "#bedb76";
    ctx.beginPath();
    ctx.ellipse(780 - offset, 90, 150, 150, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#718d50";
    ctx.beginPath();
    ctx.moveTo(-100, H);
    ctx.bezierCurveTo(60 - offset, 110, 420 - offset, 90, 710 - offset, H);
    ctx.fill();
    ctx.fillStyle = "#365f41";
    ctx.beginPath();
    ctx.moveTo(-50, H);
    ctx.bezierCurveTo(
      410 - offset,
      H + 10,
      470 - offset,
      180,
      1100 - offset,
      250,
    );
    ctx.lineTo(1100, H);
    ctx.fill();
    ctx.fillStyle = "#1e3e2d";
    ctx.beginPath();
    ctx.moveTo(350, H);
    ctx.bezierCurveTo(700 - offset, 250, 930 - offset, 350, 1100, H);
    ctx.fill();
    if (d.kind === "phone") {
      ctx.fillStyle = "#f3f7df";
      ctx.font = "500 58px sans-serif";
      ctx.textAlign = "center";
      ctx.fillText("9:41", 480, 190);
      rounded(ctx, 110, H * 0.44, 740, 230, 50, "#f5f7ed");
      ctx.fillStyle = "#2e5339";
      ctx.font = "500 45px sans-serif";
      ctx.fillText(
        mode === "control" ? "Your touch." : "Room to breathe.",
        480,
        H * 0.44 + 105,
      );
      ctx.fillStyle = "#8eaa83";
      ctx.fillRect(240, H * 0.44 + 155, 480, 10);
      ctx.textAlign = "left";
      rounded(ctx, 320, H - 55, 320, 10, 5, "#e4efd7");
    } else {
      const travel =
        mode === "extend"
          ? (Math.sin(clock * 0.55) * 0.5 + 0.5) * 660 - offset
          : 0;
      const phase = (Math.sin(clock * 0.48 - Math.PI / 2) + 1) * 0.5;
      const wx = mode === "extend" ? 100 + travel : 115 + phase * 185,
        wy = mode === "control" ? 90 : 88 + phase * 30;
      ctx.save();
      ctx.shadowColor = "#132f3330";
      ctx.shadowBlur = 35;
      ctx.shadowOffsetY = 14;
      rounded(ctx, wx, wy, 420, 260, 16, "#fafbf2");
      ctx.restore();
      rounded(ctx, wx + 23, wy + 22, 82, 7, 3, "#cbd6be");
      ctx.fillStyle = "#294c32";
      ctx.font = "500 43px sans-serif";
      ctx.fillText("Make room", wx + 32, wy + 104);
      ctx.fillText("for a good idea.", wx + 32, wy + 158);
      rounded(ctx, wx + 32, wy + 199, 290, 6, 3, "#ccd7c3");
      rounded(ctx, wx + 32, wy + 220, 220, 6, 3, "#dce2d1");
      if (mode === "control") {
        const px = 360 + Math.sin(clock * 0.65) * 100,
          py = 200 + Math.cos(clock * 0.65) * 65;
        ctx.fillStyle = "#b9e66b";
        ctx.strokeStyle = "#24452e";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.moveTo(px, py);
        ctx.lineTo(px + 20, py + 63);
        ctx.lineTo(px + 34, py + 39);
        ctx.lineTo(px + 63, py + 29);
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
        rounded(ctx, px + 36, py + 45, 125, 38, 10, "#24452e");
        ctx.fillStyle = "#fff";
        ctx.font = "20px sans-serif";
        ctx.fillText("You", px + 54, py + 71);
      }
      rounded(ctx, 350, H - 48, 260, 34, 12, "#f6f7edcc");
      for (let n = 0; n < 5; n++)
        rounded(
          ctx,
          370 + n * 46,
          H - 40,
          21,
          18,
          5,
          n === 2 ? "#284b35" : "#a6be91",
        );
    }
    d.texture.needsUpdate = true;
  }
  function resize() {
    const w = mount.clientWidth,
      h = mount.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.fov = w < 600 ? 38 : 30;
    camera.position.z =
      w < 600
        ? 15
        : Math.max(8.8, 13 / (2 * Math.tan(Math.PI / 12) * camera.aspect));
    camera.position.y = 1.4 + camera.position.z * 0.25;
    camera.lookAt(0, 1.4, 0);
    camera.updateProjectionMatrix();
    draw(0, true);
  }
  function draw(now, once = false) {
    // A synchronous resize redraw does not consume the pending animation frame.
    // Clearing its ID here would let wake() start a second, uncancellable loop.
    if (!once) frame = 0;
    const running = !paused && !reduced.matches && visible && !document.hidden;
    if (!once && running && last && now - last < 33) {
      frame = requestAnimationFrame(draw);
      return;
    }
    if (last && running && !once)
      clock += Math.max(0, Math.min((now - last) / 1000, 0.1));
    if (!once) last = now;
    const oldDeskX = desk.rotation.x,
      oldDeskY = desk.rotation.y;
    desk.rotation.y = T.MathUtils.lerp(
      desk.rotation.y,
      targetX,
      paused || reduced.matches ? 1 : 0.065,
    );
    desk.rotation.x = T.MathUtils.lerp(
      desk.rotation.x,
      targetY,
      paused || reduced.matches ? 1 : 0.065,
    );
    let moved =
      Math.abs(oldDeskX - desk.rotation.x) +
        Math.abs(oldDeskY - desk.rotation.y) >
      0.0001;
    devices.forEach((d, i) => {
      const oldX = d.group.position.x,
        oldAngle = d.group.rotation.y;
      const spread = mode === "extend" ? 1.1 : 1;
      const desired =
        d.base.x * spread +
        (layout === 1
          ? i === 2
            ? -1.2
            : 0.2
          : layout === 2
            ? i === 2
              ? 1.3
              : -0.2
            : 0);
      d.group.position.x = T.MathUtils.lerp(
        d.group.position.x,
        desired,
        reduced.matches || paused ? 1 : 0.07,
      );
      d.group.rotation.y = T.MathUtils.lerp(
        d.group.rotation.y,
        (i === 0 ? 0.24 : i === 1 ? -0.35 : -0.16) +
          (layout === 1 ? 0.16 : layout === 2 ? -0.12 : 0),
        reduced.matches || paused ? 1 : 0.07,
      );
      moved ||=
        Math.abs(oldX - d.group.position.x) +
          Math.abs(oldAngle - d.group.rotation.y) >
        0.0001;
      paint(d, i);
    });
    // Moving pixels do not change an opaque device's shadow. Recompute the
    // shadow map only when the physical composition changes.
    if (moved) renderer.shadowMap.needsUpdate = true;
    renderer.render(scene, camera);
    if (running && !once) frame = requestAnimationFrame(draw);
  }
  function wake() {
    if (lost) return;
    if (frame) cancelAnimationFrame(frame);
    last = 0;
    frame = requestAnimationFrame(draw);
  }
  window.addEventListener("mm-preview-mode", (e) => {
    mode = e.detail;
    wake();
  });
  mount.addEventListener("pointermove", (e) => {
    if (reduced.matches || e.pointerType !== "mouse") return;
    const r = mount.getBoundingClientRect();
    targetX = ((e.clientX - r.left - r.width / 2) / r.width) * 0.19;
    targetY = ((e.clientY - r.top - r.height / 2) / r.height) * 0.055;
    if (!frame) wake();
  });
  mount.addEventListener("pointerleave", () => {
    targetX = targetY = 0;
    wake();
  });
  rearrange?.addEventListener("click", () => {
    layout = (layout + 1) % 3;
    wake();
  });
  new MutationObserver(() => {
    paused = document.body.classList.contains("motion-paused");
    wake();
  }).observe(document.body, { attributes: true, attributeFilter: ["class"] });
  new IntersectionObserver(
    (entries) => {
      visible = entries[0].isIntersecting;
      wake();
    },
    { rootMargin: "80px" },
  ).observe(mount);
  document.addEventListener("visibilitychange", wake);
  reduced.addEventListener("change", wake);
  new ResizeObserver(resize).observe(mount);
  resize();
  wake();
  renderer.domElement.addEventListener("webglcontextlost", (e) => {
    e.preventDefault();
    lost = true;
    if (frame) cancelAnimationFrame(frame);
    mount.classList.remove("rendered");
  });
  renderer.domElement.addEventListener("webglcontextrestored", () => {
    lost = false;
    mount.classList.add("rendered");
    wake();
  });
})();
