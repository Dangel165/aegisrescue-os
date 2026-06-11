#!/usr/bin/env bash
# Live ISO GUI stack — LightDM + Openbox + Tauri runtime (AegisCenter / Installer)
configure_live_gui() {
    local chroot="$1"

    log "Live GUI 패키지 설치 중 (LightDM, Xorg, WebKitGTK)..."

    chroot "${chroot}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get install -y --no-install-recommends \
        xserver-xorg-core xserver-xorg-video-all \
        x11-xserver-utils x11-xkb-utils xauth \
        lightdm openbox obconf nitrogen \
        libwebkit2gtk-4.1-0 libjavascriptcoregtk-4.1-0 \
        libgtk-3-0 libayatana-appindicator3-1 librsvg2-2 \
        libgl1 libegl1 libgbm1 libxcomposite1 libxdamage1 libxrandr2 \
        libasound2t64 libssl3 \
        fonts-dejavu-core fontconfig dbus-x11 \
        2>&1 | tail -15 || warn "일부 GUI 패키지 설치 실패"

    chroot "${chroot}" bash -c \
        'echo "lightdm shared/default-x-display-manager select lightdm" | debconf-set-selections; \
         echo "lightdm shared/default-x-display-manager seen true" | debconf-set-selections; \
         DEBIAN_FRONTEND=noninteractive dpkg-reconfigure -f noninteractive lightdm' \
        2>/dev/null || true

    mkdir -p "${chroot}/etc/lightdm/lightdm.conf.d"
    cat > "${chroot}/etc/lightdm/lightdm.conf.d/aegis.conf" <<'EOF'
[Seat:*]
autologin-user=aegis
autologin-user-timeout=0
autologin-session=openbox
EOF

    mkdir -p "${chroot}/usr/share/applications"
    cat > "${chroot}/usr/share/applications/aegis-installer.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=AegisRescue Installer
Comment=Install AegisRescue to disk or USB
Exec=aegis-installer
Terminal=false
Categories=System;
EOF

    cat > "${chroot}/usr/share/applications/aegiscenter.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=AegisCenter
Comment=AegisRescue recovery control panel
Exec=aegiscenter
Terminal=false
Categories=System;
EOF

    chroot "${chroot}" usermod -aG video,input,render aegis 2>/dev/null || true

    chroot "${chroot}" systemctl enable lightdm.service 2>/dev/null || true
    chroot "${chroot}" systemctl set-default graphical.target 2>/dev/null || true
}

install_live_gui_scripts() {
    local chroot="$1"
    local project_root="$2"

    mkdir -p "${chroot}/opt/aegis/bin"
    cp "${project_root}/packaging/scripts/aegis-live-autostart.sh" \
       "${chroot}/opt/aegis/bin/aegis-live-autostart.sh"
    chmod 755 "${chroot}/opt/aegis/bin/aegis-live-autostart.sh"

    mkdir -p "${chroot}/home/aegis/.config/openbox" "${chroot}/etc/skel/.config/openbox"
    ln -sf /opt/aegis/bin/aegis-live-autostart.sh \
        "${chroot}/home/aegis/.config/openbox/autostart"
    ln -sf /opt/aegis/bin/aegis-live-autostart.sh \
        "${chroot}/etc/skel/.config/openbox/autostart"
    chroot "${chroot}" chown -R aegis:aegis /home/aegis/.config 2>/dev/null || true
}
