<script>
  import Router from "svelte-spa-router";
  import Dashboard from "./routes/Dashboard.svelte";

  import Login from "./routes/Login.svelte";

  const routes = {
    "/": Login,
    "/dashboard": Dashboard,

    // Catch-all
    // This is optional, but if present it must be the last
    "*": Login,
  };
</script>

<Router {routes} />
