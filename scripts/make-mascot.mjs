#!/usr/bin/env node
// Пересборка спрайтов маскота (ведьмы) для hero-секции главной.
//
// Маскот — это пять кадров одной позы: базовая эмоция и четыре реакции на
// действия терминала (см. `assets/mascot/`). Кадры обязаны совпадать по
// размеру холста и по точке опоры: иначе при смене эмоции ведьма прыгает.
// Поэтому холст, масштаб и выравнивание считаются здесь один раз и
// применяются ко всем пяти картинкам.
//
// Использование:
//   node scripts/make-mascot.mjs [каталог-с-исходниками] [ширина] [высота]
//
// Каталог по умолчанию — `.fool-work/mascot/source`; в нём должны лежать
// base.png, dissatisfied.png, read.png, thinking.png, smirk.png. Исходники
// живут вне репозитория (как и у обложки og): это разовые входные данные
// художника, а не часть сборки. Скрипт — единственное место, где известен
// способ их обработки, и он повторяем: тот же вход даёт те же файлы.
//
// Что делает с каждой картинкой:
//   1. обрезает прозрачные поля по альфе;
//   2. уменьшает до холста усреднением по площади (box-фильтр) — усредняем
//      premultiplied RGB, иначе на краях остаётся тёмный ореол от прозрачных
//      пикселей;
//   3. делает альфу бинарной: пиксель либо есть, либо его нет. Полупрозрачные
//      края выдают «нарисованность» и мешают `image-rendering: pixelated`;
//   4. выравнивает по низу и центру холста: низ спрайта — подошвы, они у всех
//      кадров на месте. Общая точка опоры важнее красивых полей: маскот стоит
//      в своей полосе между чатом и описанием, и сдвиг на пару пикселей при
//      смене эмоции читается как дрожь.
//
// Зависимостей нет: PNG читается и пишется встроенным zlib. ImageMagick для
// этой задачи не нужен, а палитра у источника и так маленькая (35–58 цветов).

import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/// Холст всех кадров по умолчанию: 144×192 логических пикселя.
///
/// Это верхняя граница полезного разрешения источника (он около 234×317):
/// 192/317 ≈ 0,61 — уменьшение меньше чем вдвое, поэтому детали (лицо, блики
/// на шляпе, застёжка) не смазываются. Пропорции 3:4 держат все размеры на
/// странице кратными: CSS показывает кадр в 144, 120 или 96px по ширине, это
/// ровно 1×, 5/6 и 2/3 от холста — без дробных пикселей у пиксель-арта.
///
/// Размеры можно переопределить аргументами: если однажды понадобится другой
/// холст, разметку правит `MASCOT_WIDTH`/`MASCOT_HEIGHT` в `api/assets.rs`.
const CANVAS_WIDTH = Number(process.argv[3]) || 144;
const CANVAS_HEIGHT = Number(process.argv[4]) || 192;

/// Порог альфы при обрезке и бинаризации. 128 — середина: пиксель, который
/// после усреднения остался прозрачнее половины, пропадает, иначе появляется.
const ALPHA_THRESHOLD = 128;

const EMOTIONS = ['base', 'dissatisfied', 'read', 'thinking', 'smirk'];
const SOURCE_DIR = process.argv[2] || '.fool-work/mascot/source';
const OUT_DIR = 'assets/mascot';

/// Декодирует PNG в RGBA8.
///
/// Поддерживаются только 8-битные неинтерлейсные файлы: источник готовится
/// художником, и требование «один формат» здесь дешевле универсальности.
function decodePng(file) {
    const buf = fs.readFileSync(file);
    if (!buf.subarray(0, 8).equals(SIGNATURE)) {
        throw new Error(`${file}: это не PNG`);
    }
    let offset = 8;
    let header = null;
    const idat = [];
    let palette = null;
    let transparency = null;
    while (offset < buf.length) {
        const length = buf.readUInt32BE(offset);
        const type = buf.toString('ascii', offset + 4, offset + 8);
        const data = buf.subarray(offset + 8, offset + 8 + length);
        if (type === 'IHDR') {
            header = {
                width: data.readUInt32BE(0),
                height: data.readUInt32BE(4),
                depth: data[8],
                color: data[9],
                interlace: data[12],
            };
        } else if (type === 'IDAT') {
            idat.push(data);
        } else if (type === 'PLTE') {
            palette = Buffer.from(data);
        } else if (type === 'tRNS') {
            transparency = Buffer.from(data);
        }
        offset += 12 + length;
    }
    if (!header) throw new Error(`${file}: нет IHDR`);
    if (header.depth !== 8 || header.interlace) {
        throw new Error(`${file}: нужен 8-битный неинтерлейсный PNG`);
    }
    const channels = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[header.color];
    if (!channels) throw new Error(`${file}: тип цвета ${header.color} не поддержан`);

    const { width, height } = header;
    const raw = zlib.inflateSync(Buffer.concat(idat));
    const stride = width * channels;
    const pixels = Buffer.alloc(stride * height);
    let previous = Buffer.alloc(stride);
    for (let y = 0; y < height; y += 1) {
        const filter = raw[y * (stride + 1)];
        const line = Buffer.from(
            raw.subarray(y * (stride + 1) + 1, y * (stride + 1) + 1 + stride),
        );
        for (let i = 0; i < stride; i += 1) {
            const left = i >= channels ? line[i - channels] : 0;
            const up = previous[i];
            const upLeft = i >= channels ? previous[i - channels] : 0;
            if (filter === 1) line[i] = (line[i] + left) & 0xff;
            else if (filter === 2) line[i] = (line[i] + up) & 0xff;
            else if (filter === 3) line[i] = (line[i] + ((left + up) >> 1)) & 0xff;
            else if (filter === 4) {
                const p = left + up - upLeft;
                const pa = Math.abs(p - left);
                const pb = Math.abs(p - up);
                const pc = Math.abs(p - upLeft);
                const predictor = pa <= pb && pa <= pc ? left : pb <= pc ? up : upLeft;
                line[i] = (line[i] + predictor) & 0xff;
            } else if (filter !== 0) {
                throw new Error(`${file}: фильтр ${filter} не поддержан`);
            }
        }
        line.copy(pixels, y * stride);
        previous = line;
    }

    const rgba = Buffer.alloc(width * height * 4);
    for (let i = 0; i < width * height; i += 1) {
        if (header.color === 6) {
            pixels.copy(rgba, i * 4, i * 4, i * 4 + 4);
        } else if (header.color === 2) {
            pixels.copy(rgba, i * 4, i * 3, i * 3 + 3);
            rgba[i * 4 + 3] = 255;
        } else if (header.color === 0) {
            rgba.fill(pixels[i], i * 4, i * 4 + 3);
            rgba[i * 4 + 3] = 255;
        } else if (header.color === 4) {
            rgba.fill(pixels[i * 2], i * 4, i * 4 + 3);
            rgba[i * 4 + 3] = pixels[i * 2 + 1];
        } else if (header.color === 3) {
            const at = pixels[i] * 3;
            rgba[i * 4] = palette[at];
            rgba[i * 4 + 1] = palette[at + 1];
            rgba[i * 4 + 2] = palette[at + 2];
            rgba[i * 4 + 3] = transparency && transparency[pixels[i]] != null
                ? transparency[pixels[i]]
                : 255;
        }
    }
    return { width, height, rgba };
}

let crcTable = null;

function crc32(buf) {
    if (!crcTable) {
        crcTable = new Int32Array(256);
        for (let n = 0; n < 256; n += 1) {
            let c = n;
            for (let k = 0; k < 8; k += 1) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
            crcTable[n] = c;
        }
    }
    let crc = -1;
    for (let i = 0; i < buf.length; i += 1) crc = (crc >>> 8) ^ crcTable[(crc ^ buf[i]) & 0xff];
    return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, crc]);
}

/// Кодирует RGBA8 в PNG. Фильтров не применяем: картинка 144×192, экономить
/// нечего, а предсказание только запутало бы сравнение версий.
function encodePng({ width, height, rgba }, file) {
    const stride = width * 4;
    const raw = Buffer.alloc((stride + 1) * height);
    for (let y = 0; y < height; y += 1) {
        rgba.copy(raw, y * (stride + 1) + 1, y * stride, y * stride + stride);
    }
    const header = Buffer.alloc(13);
    header.writeUInt32BE(width, 0);
    header.writeUInt32BE(height, 4);
    header[8] = 8;
    header[9] = 6;
    const png = Buffer.concat([
        SIGNATURE,
        chunk('IHDR', header),
        chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
        chunk('IEND', Buffer.alloc(0)),
    ]);
    fs.writeFileSync(file, png);
    return png.length;
}

/// Границы непрозрачного содержимого.
function contentBox({ width, height, rgba }) {
    let minX = width;
    let minY = height;
    let maxX = -1;
    let maxY = -1;
    for (let y = 0; y < height; y += 1) {
        for (let x = 0; x < width; x += 1) {
            if (rgba[(y * width + x) * 4 + 3] < ALPHA_THRESHOLD) continue;
            if (x < minX) minX = x;
            if (y < minY) minY = y;
            if (x > maxX) maxX = x;
            if (y > maxY) maxY = y;
        }
    }
    if (maxX < 0) throw new Error('пустая картинка: нет непрозрачных пикселей');
    return { x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1 };
}

/// Уменьшает обрезанную картинку до `scale` и кладёт её на холст.
///
/// Усреднение по площади: каждый пиксель холста — среднее всех исходных
/// пикселей, которые в него попали. Для альфы и цвета отдельно нельзя
/// (прозрачные пиксели тянут цвет к своему), поэтому RGB усредняется
/// premultiplied, а потом делится на альфу.
function placeOnCanvas(image, box, scale) {
    const canvas = Buffer.alloc(CANVAS_WIDTH * CANVAS_HEIGHT * 4);
    const outWidth = Math.max(1, Math.round(box.width * scale));
    const outHeight = Math.max(1, Math.round(box.height * scale));
    const offsetX = Math.round((CANVAS_WIDTH - outWidth) / 2);
    const offsetY = CANVAS_HEIGHT - outHeight;

    for (let y = 0; y < outHeight; y += 1) {
        const srcY0 = box.y + Math.floor((y * box.height) / outHeight);
        const srcY1 = box.y + Math.max(
            srcY0 - box.y + 1,
            Math.ceil(((y + 1) * box.height) / outHeight),
        );
        for (let x = 0; x < outWidth; x += 1) {
            const srcX0 = box.x + Math.floor((x * box.width) / outWidth);
            const srcX1 = box.x + Math.max(
                srcX0 - box.x + 1,
                Math.ceil(((x + 1) * box.width) / outWidth),
            );
            let alphaSum = 0;
            let redSum = 0;
            let greenSum = 0;
            let blueSum = 0;
            let count = 0;
            for (let sy = srcY0; sy < srcY1; sy += 1) {
                for (let sx = srcX0; sx < srcX1; sx += 1) {
                    const at = (sy * image.width + sx) * 4;
                    const alpha = image.rgba[at + 3] / 255;
                    alphaSum += alpha;
                    redSum += image.rgba[at] * alpha;
                    greenSum += image.rgba[at + 1] * alpha;
                    blueSum += image.rgba[at + 2] * alpha;
                    count += 1;
                }
            }
            const alpha = alphaSum / count;
            const at = ((offsetY + y) * CANVAS_WIDTH + offsetX + x) * 4;
            if (alpha * 255 < ALPHA_THRESHOLD) continue; // дырка: пикселя нет
            canvas[at] = Math.round(redSum / alphaSum);
            canvas[at + 1] = Math.round(greenSum / alphaSum);
            canvas[at + 2] = Math.round(blueSum / alphaSum);
            canvas[at + 3] = 255;
        }
    }
    return { width: CANVAS_WIDTH, height: CANVAS_HEIGHT, rgba: canvas };
}

const sources = EMOTIONS.map((emotion) => {
    const file = path.join(SOURCE_DIR, `${emotion}.png`);
    const image = decodePng(file);
    const box = contentBox(image);
    return { emotion, file, image, box };
});

// Масштаб общий для всех кадров — по самому высокому. Иначе высокая поза
// (ведьма подалась вперёд) оказалась бы крупнее остальных, и при смене эмоции
// персонаж менял бы размер.
const tallest = Math.max(...sources.map((s) => s.box.height));
const widest = Math.max(...sources.map((s) => s.box.width));
const scale = Math.min(CANVAS_HEIGHT / tallest, CANVAS_WIDTH / widest);

fs.mkdirSync(OUT_DIR, { recursive: true });
const report = [];
for (const source of sources) {
    const canvas = placeOnCanvas(source.image, source.box, scale);
    const colors = new Set();
    for (let i = 0; i < CANVAS_WIDTH * CANVAS_HEIGHT; i += 1) {
        if (canvas.rgba[i * 4 + 3] === 0) continue;
        colors.add(
            (canvas.rgba[i * 4] << 16) | (canvas.rgba[i * 4 + 1] << 8) | canvas.rgba[i * 4 + 2],
        );
    }
    const out = path.join(OUT_DIR, `${source.emotion}.png`);
    const bytes = encodePng(canvas, out);
    report.push(
        `${source.emotion.padEnd(13)} ${source.box.width}×${source.box.height} → `
        + `${CANVAS_WIDTH}×${CANVAS_HEIGHT}, ${colors.size} цветов, ${(bytes / 1024).toFixed(1)} КБ`,
    );
}
console.log(`Холст ${CANVAS_WIDTH}×${CANVAS_HEIGHT}, масштаб ${scale.toFixed(4)}`);
console.log(report.join('\n'));
