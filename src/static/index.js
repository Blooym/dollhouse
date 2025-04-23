"use strict"

function getTokenInput() {
    return document.getElementById("token");
}

function getStoredToken() {
    return localStorage.getItem("token");
}

function setStoredToken() {
    token = getTokenInput().value;
    localStorage.setItem("token", token);
}

const errorText = document.getElementById("error");
const fileUploadForm = document.getElementById("fileUploadForm");

let token = getStoredToken();
if (token) {
    getTokenInput().value = token;
}
fileUploadForm.addEventListener("submit", uploadFile);

// File upload handler.
async function uploadFile(event) {
    event.preventDefault();
    const fileInput = document.getElementById('fileInput');
    if (fileInput.files.length === 0) {
        errorText.innerText = "No files selected to upload.";
        throw new Error("no files selected to upload.");
    }

    const fileUploadButton = document.getElementById("fileUploadButton");
    const oldUploadText = fileUploadButton.innerText;
    fileUploadButton.innerText = "Processing";
    fileUploadButton.disabled = true;

    const formData = new FormData();
    for (const file of fileInput.files) {
        formData.append("files", file);
    }

    try {
        const res = await fetch(`${window.location.protocol}//${window.location.host}/upload`, {
            method: "POST",
            body: formData,
            headers: {
                "Authorization": `Bearer ${token}`,
            }
        });
        if (!res.ok) {
            throw new Error(`failed to upload media: ${res.status} - ${res.statusText}`);
        }

        const json = await res.json();
        const redirectUrl = json["url"];
        if (redirectUrl === undefined || redirectUrl === null) {
            throw new Error("server returned malformed response object");
        }
        fileUploadButton.innerText = oldUploadText;
        fileUploadButton.disabled = false;
        fileInput.value = null;

        if (!navigator.share) {
            await navigator.clipboard.writeText(redirectUrl);
            alert("Upload url copied to clipboard")
        } else {
            try {
                await navigator.share({
                    title: "Share this upload",
                    url: redirectUrl
                });
            } catch { }
        }

    } catch (e) {
        fileUploadButton.innerText = oldUploadText;
        fileUploadButton.disabled = false;
        errorText.innerText = e.toString();
        console.error(e);
    }
}