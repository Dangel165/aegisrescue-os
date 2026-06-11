#!/usr/bin/env bash
# Live ISO GUI — LightDM autologin → Openbox → installer (Ubuntu-style)
configure_live_gui() {
    local chroot="$1"
    local project_root="${2:-}"

    log "Live GUI 패키지 설치 중 (LightDM, Xorg, Openbox, WebKitGTK)..."

    chroot "${chroot}" env DEBIAN_FRONTEND=noninteractive TZ=Etc/UTC \
        apt-get install -y --no-install-recommends \
        lightdm lightdm-gtk-greeter \
        xserver-xorg-core xserver-xorg xserver-xorg-legacy \
        xserver-xorg-video-vesa xserver-xorg-video-fbdev \
        xserver-xorg-video-vmware xserver-xorg-video-modesetting \
        xserver-xorg-video-all \
        xserver-xorg-input-libinput \
        x11-xserver-utils x11-xkb-utils xauth xinit x11-common \
        openbox obconf nitrogen \
        libwebkit2gtk-4.1-0 libjavascriptcoregtk-4.1-0 \
        libgtk-3-0 libayatana-appindicator3-1 librsvg2-2 \
        libgl1 libegl1 libgbm1 libxcomposite1 libxdamage1 libxrandr2 \
        libasound2t64 libssl3 \
        fonts-dejavu-core fonts-liberation fontconfig \
        fonts-noto-core fonts-noto-cjk fonts-noto-cjk-extra fonts-nanum \
        fonts-noto-color-emoji \
        dbus-x11 \
        2>&1 | tail -15 || warn "일부 GUI 패키지 설치 실패"

    chroot "${chroot}" fc-cache -f 2>/dev/null || true

    mkdir -p "${chroot}/etc/fonts/conf.d"
    cat > "${chroot}/etc/fonts/conf.d/65-aegis-cjk.conf" <<'EOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
  <alias binding="strong">
    <family>sans-serif</family>
    <prefer>
      <family>Noto Sans CJK KR</family>
      <family>Noto Sans CJK JP</family>
      <family>Noto Sans CJK SC</family>
      <family>Nanum Gothic</family>
      <family>DejaVu Sans</family>
    </prefer>
  </alias>
  <alias binding="strong">
    <family>emoji</family>
    <prefer>
      <family>Noto Color Emoji</family>
    </prefer>
  </alias>
</fontconfig>
EOF

    chroot "${chroot}" locale-gen ko_KR.UTF-8 en_US.UTF-8 ja_JP.UTF-8 zh_CN.UTF-8 2>/dev/null || true

    rm -f "${chroot}/etc/X11/xorg.conf.d/99-aegis-vm.conf" 2>/dev/null || true

    mkdir -p "${chroot}/etc/modules-load.d"
    cat > "${chroot}/etc/modules-load.d/aegis-vm-gfx.conf" <<'EOF'
# VirtualBox VMSVGA / VMware
vmwgfx
drm_kms_helper
EOF

    mkdir -p "${chroot}/etc/X11"
    cat > "${chroot}/etc/X11/Xwrapper.config" <<'EOF'
allowed_users=anybody
needs_root_rights=yes
EOF

    {
        echo 'xserver-xorg-legacy xserver-xorg-legacy/xwrapper/allowed_users select Anybody'
        echo 'xserver-xorg-legacy xserver-xorg-legacy/xwrapper/allowed_users seen true'
    } | chroot "${chroot}" debconf-set-selections 2>/dev/null || true
    chroot "${chroot}" env DEBIAN_FRONTEND=noninteractive \
        dpkg-reconfigure -f noninteractive xserver-xorg-legacy 2>/dev/null || true
    cat > "${chroot}/etc/X11/Xwrapper.config" <<'EOF'
allowed_users=anybody
needs_root_rights=yes
EOF

    echo '/usr/sbin/lightdm' > "${chroot}/etc/X11/default-display-manager"

    # debconf: LightDM as default DM + autologin user
    {
        echo 'shared shared/default-x-display-manager select lightdm'
        echo 'shared shared/default-x-display-manager seen true'
        echo 'lightdm lightdm/autologin-user string aegis'
        echo 'lightdm lightdm/autologin-user seen true'
    } | chroot "${chroot}" debconf-set-selections 2>/dev/null || true
    chroot "${chroot}" env DEBIAN_FRONTEND=noninteractive \
        dpkg-reconfigure -f noninteractive lightdm 2>/dev/null || true

    # LightDM — autologin aegis → Openbox (installer via ~/.config/openbox/autostart)
    mkdir -p "${chroot}/etc/lightdm/lightdm.conf.d"
    mkdir -p "${chroot}/usr/share/xsessions"
    if [[ -n "$project_root" && -f "${project_root}/packaging/lightdm/aegis-lightdm.conf" ]]; then
        cp "${project_root}/packaging/lightdm/aegis-lightdm.conf" \
           "${chroot}/etc/lightdm/lightdm.conf.d/50-aegis.conf"
    fi

    mkdir -p "${chroot}/var/log/lightdm"

    mkdir -p "${chroot}/usr/share/applications"
    cat > "${chroot}/usr/share/applications/aegis-installer.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=AegisRescue Installer
Comment=Install AegisRescue to disk or USB
Exec=/opt/aegis/bin/aegis-installer-gui.sh
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

    chroot "${chroot}" usermod -aG video,input,render,aegis aegis 2>/dev/null || true

    mkdir -p "${chroot}/etc/tmpfiles.d"
    cat > "${chroot}/etc/tmpfiles.d/aegis-run.conf" <<'EOF'
d /run/aegis 0775 root aegis -
EOF

    chroot "${chroot}" systemctl enable getty@tty2.service 2>/dev/null || true

    # GUI Live: graphical.target + LightDM (CLI boot uses multi-user.target via kernel cmdline)
    chroot "${chroot}" systemctl set-default graphical.target 2>/dev/null || true
    chroot "${chroot}" systemctl unmask lightdm.service 2>/dev/null || true
    chroot "${chroot}" systemctl enable lightdm.service 2>/dev/null || true
    chroot "${chroot}" systemctl enable aegis-gui-fallback.service 2>/dev/null || true

    # Installer runs from Openbox autostart (user session). System unit cannot access :0 → restart loop.
    chroot "${chroot}" systemctl disable aegis-installer.service 2>/dev/null || true
    chroot "${chroot}" systemctl mask aegis-installer.service 2>/dev/null || true

    # Plymouth handoff to LightDM (like Ubuntu)
    chroot "${chroot}" systemctl unmask plymouth-quit-wait.service 2>/dev/null || true

    # Remove legacy startx-on-login hook
    rm -f "${chroot}/etc/profile.d/aegis-start-gui.sh" 2>/dev/null || true

    chroot "${chroot}" plymouth-set-default-theme aegisrescue 2>/dev/null || true
}

install_live_gui_scripts() {
    local chroot="$1"
    local project_root="$2"

    mkdir -p "${chroot}/opt/aegis/bin"
    for script in aegis-live-autostart.sh aegis-installer-gui.sh aegis-xsession.sh aegis-startx.sh aegis-gui-fallback.sh; do
        cp "${project_root}/packaging/scripts/${script}" \
           "${chroot}/opt/aegis/bin/${script}"
        chmod 755 "${chroot}/opt/aegis/bin/${script}"
    done

    cp "${project_root}/packaging/systemd/aegis-gui-fallback.service" \
       "${chroot}/etc/systemd/system/aegis-gui-fallback.service" 2>/dev/null || true

    mkdir -p "${chroot}/home/aegis/.config/openbox" \
             "${chroot}/etc/skel/.config/openbox"
    ln -sf /opt/aegis/bin/aegis-live-autostart.sh \
        "${chroot}/home/aegis/.config/openbox/autostart"
    ln -sf /opt/aegis/bin/aegis-live-autostart.sh \
        "${chroot}/etc/skel/.config/openbox/autostart"

    chroot "${chroot}" chown -R aegis:aegis /home/aegis 2>/dev/null || true
}
