<script>
  import { onMount } from "svelte";
  import { user, sendWsMessage, ws, isLoggedIn } from "../store";
  import Input from "../components/InputLogin.svelte";
  import { push } from "svelte-spa-router";

  // I guess check if user is already logged in?
  onMount(() => {
    user.subscribe((something) => {
      console.log("Updated", something);
    });
  });

  $: if ($isLoggedIn) {
    push("/dashboard");
  }

  let username;
  let password;

  async function login() {
    let res = await sendWsMessage({
      type: "auth",
      username,
      password,
    });

    console.log("GOT RESSSS POG", res);

    let config = await sendWsMessage({
      type: "me",
    });

    console.log("GOT RESSSS POG", config);
    $user = config;
  }
</script>

<div class="min-h-screen flex items-center justify-center py-10 px-5">
  <div class="max-w-md w-full space-y-8">
    <h1 class="text-center text-3xl font-extrabold text-skin-base">
      Sign in to NOALBS
    </h1>

    <form
      on:submit|preventDefault={login}
      class="mt-8 space-y-6"
      autocomplete="off"
    >
      <div class="rounded-md -space-y-px shadow">
        <Input
          bind:value={username}
          label="username"
          type="text"
          placeholder="Username"
          class="w-full relative border border-gray-300 rounded-t-md focus:outline-none focus:ring-gray-700 focus:border-gray-700 focus:z-10"
        />
        <Input
          bind:value={password}
          label="password"
          type="password"
          placeholder="Password"
          class="w-full relative border border-gray-300 rounded-b-md focus:outline-none focus:ring-gray-700 focus:border-gray-700 focus:z-10"
        />
      </div>

      <div class="flex items-center">
        <input
          id="remember"
          name="remember"
          type="checkbox"
          class="h-4 w-4 text-gray-800 focus:ring-gray-700 border-gray-300 rounded"
        />
        <label for="remember" class="ml-2 block text-sm text-skin-base">
          Remember me
        </label>
      </div>

      <button
        type="submit"
        class="w-full flex justify-center py-2 px-3 rounded-md font-bold bg-skin-button-accent hover:bg-skin-button-hover text-skin-button focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-700"
        >Sign in
      </button>
    </form>
  </div>
</div>
