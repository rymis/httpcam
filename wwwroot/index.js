function time() {
    return Date.now() / 1000.0;
}

function sleep(t) {
    return new Promise(function (resolve) {
        setTimeout(resolve, t);
    }); 
}

async function sleepUntil(t) {
    let now = time();
    if (now < t) {
        await sleep(t - now);
    }

    return;
}

function preloadImage(url) {
    let img = new Image();
    img.src = url;
    return new Promise(function (resolve, reject) {
        img.onload = function () {
            resolve(img);
        }
        img.onerror = function () {
            reject("Can't load image");
        }
    });
}

async function updateImage() {
    let seq = 0;
    while (true) {
        let rnd = Math.round(Math.random() * 1000000 + 1.0);
        let src = "/image.jpg?seq=" + seq + "&rnd=" + rnd;
        ++seq;
        let img = await preloadImage(src);
        let old = document.getElementById("webcam");
        old.src = src;

        await sleep(0.1);
    }
}

document.body.onload = function() {
    updateImage().then(function () { console.log("Error"); });
}
