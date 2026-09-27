fn main() {
    topcoat::icon::iconify::BuildConfig::new().icon_set("lucide").stage().unwrap();
    topcoat::tailwind::BuildConfig::new().input("styles.css").output("src/tailwind.css").render().unwrap();
}
