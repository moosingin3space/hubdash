document.addEventListener('alpine:init', () => {
    Alpine.data('expandable', (fetchUrl, targetId) => ({
        expanded: false,
        loaded: false,

        toggle() {
            this.expanded = !this.expanded;
            if (this.expanded && !this.loaded) {
                htmx.ajax('GET', fetchUrl, '#' + targetId);
                this.loaded = true;
            }
        }
    }));
});
