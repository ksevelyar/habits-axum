# Nginx

```nix
services.nginx.virtualHosts."api.habits.rusty-cluster.net" = {
  forceSSL = true;
  enableACME = true;

  locations."/" = {
    proxyPass = "http://localhost:3003";
    extraConfig = ''
      proxy_set_header Host $host;
      proxy_set_header X-Real-IP $remote_addr;
      proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection "upgrade";
      proxy_read_timeout 1d;
    '';
  };
};
```

```nix
services.nginx.virtualHosts."habits.rusty-cluster.net" = {
  forceSSL = true;
  enableACME = true;

  root = inputs.habits-vue.packages.x86_64-linux.default;

  extraConfig = ''
    location / {
      try_files $uri $uri/ /index.html;
    }
  '';
};
```
