document.addEventListener('DOMContentLoaded', () => {
    document.getElementById("addressbar").addEventListener("submit", function (e) {
        e.preventDefault();
        const input = new URLSearchParams(new FormData(e.target)).get("address");
        if (input) {
            window.location.href = "/" + encodeURIComponent(input);
        }
    });
});
