<script setup lang="ts">
const props = defineProps<{
  samePadding?: boolean
}>();

const samePadding = props.samePadding ?? false;

const isDropdownOpen = ref(false);
</script>

<template>
  <div class="relative">
    <div
      @click="isDropdownOpen = !isDropdownOpen"
      class="bg-blockTransparent border-blockBorder border-1 rounded-md w-fit h-20 py-4 flex items-center justify-center backdrop-blur-sm cursor-pointer"
      :class="{
        'px-10': !samePadding,
        'px-4': samePadding
      }"
    >
      <slot />
    </div>

    <transition name="fade-align">
      <div v-if="isDropdownOpen" class="bg-blockTransparent border-blockBorder border-1 rounded-md w-fit flex items-center justify-center backdrop-blur-sm cursor-pointer absolute bottom-24 right-0">
        <slot name="dropdown" />
      </div>
    </transition>
  </div>
</template>

<style lang="scss">
@media (prefers-reduced-motion: no-preference) {
  .fade-align-enter-active,
  .fade-align-leave-active,
  .fade-align-move {
    transition: opacity 0.3s, transform 0.3s;
  }
  .fade-align-leave-active {
    @apply bottom-24 right-0 absolute
  }
  .fade-align-enter-from,
  .fade-align-leave-to {
    opacity: 0;
  }
}
</style>