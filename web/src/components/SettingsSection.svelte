<script>
  export let name;
  export let description = "";
</script>

<div>
  <div class="md:grid md:grid-cols-4 gap-2">
    <div class="px-6 md:px-0">
      <h2 class="text-lg font-medium leading-6 text-skin-base">{name}</h2>
      {#if description}
        <p class="mt-1 text-sm text-skin-muted">{description}</p>
      {/if}
    </div>

    <div class="col-span-3 mt-5 md:mt-0">
      <slot>
        <div class="card">
          <p class="text-center text-skin-base">Nothing in here :(</p>
          <p class="text-center text-skin-base">Still working on this</p>
        </div>
      </slot>
    </div>
  </div>
</div>
