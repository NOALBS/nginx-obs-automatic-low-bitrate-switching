<script>
  export let label;
  export let value;
  export let description = "";
</script>

<div class="col-span-full flex flex-col">
  <div class="flex justify-between items-center">
    <label for={label} class="text-skin-base">{label}</label>
    <input type="checkbox" id={label} bind:checked={value} />
  </div>

  {#if description}
    <p class="text-skin-muted text-sm">{description}</p>
  {/if}
</div>
