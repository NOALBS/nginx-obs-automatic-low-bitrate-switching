<script>
  export let label;
  export let value;
</script>

<div>
  <label for={label} class="sr-only">{label}</label>
  <input bind:value id={label} name={label} {...$$restProps} />
</div>
