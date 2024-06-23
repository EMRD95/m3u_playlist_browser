function lazyLoadImages() {
    var lazyloadImages = document.querySelectorAll("img.lazyload");    
    var imageObserver = new IntersectionObserver(function(entries, observer) {
        entries.forEach(function(entry) {
            if (entry.isIntersecting) {
                var image = entry.target;
                fetch(image.dataset.src)
                    .then(response => response.text())
                    .then(imageUrl => {
                        image.src = imageUrl;
                        image.classList.remove("lazyload");
                    })
                    .catch(error => {
                        console.error('Error:', error);
                        image.src = '/static/placeholder.png';
                    });
                imageObserver.unobserve(image);
            }
        });
    });

    lazyloadImages.forEach(function(image) {
        imageObserver.observe(image);
    });
}

function setView(view) {
    const channelList = document.getElementById('channelList') || document.getElementById('searchResults');
    localStorage.setItem('viewPreference', view);
    if (view === 'grid') {
        channelList.classList.add('grid-view');
        channelList.classList.remove('list-view');
    } else {
        channelList.classList.add('list-view');
        channelList.classList.remove('grid-view');
    }
}

function loadViewPreference() {
    const view = localStorage.getItem('viewPreference') || 'list';
    setView(view);
}

function playChannel(url, player) {
    fetch('/play/' + player + '/' + encodeURIComponent(url))
        .then(response => {
            if (!response.ok) {
                console.error('Failed to play channel');
            }
        })
        .catch(error => {
            console.error('Error:', error);
        });
}

document.addEventListener("DOMContentLoaded", function() {
    lazyLoadImages();
    loadViewPreference();
});
