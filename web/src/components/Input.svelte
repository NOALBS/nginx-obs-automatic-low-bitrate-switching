<script>
  export let label;
  export let value;
  export let description = "";
</script>

<div class="col-span-full sm:flex sm:justify-between space-y-2 space-y-2 md:space-y-0">
  <div class="">
    <label for={label} class="text-skin-base">{label}</label>
    {#if description}
      <p class="text-skin-muted text-sm">{description}</p>
    {/if}
  </div>

  <input
    id={label}
    bind:value
    class="rounded w-full sm:w-auto bg-skin-input text-skin-input border border-skin-input focus:outline-none focus:ring-gray-700 focus:border-gray-700 focus:z-10"
    {...$$restProps}
  />
</div>
