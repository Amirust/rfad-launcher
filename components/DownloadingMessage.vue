<script setup lang="ts">
import { computed } from 'vue';
import MessageBox from '~/components/base/MessageBox.vue';

const props = defineProps<{
  percentage: number,
  speed: string
}>();

const gradientStyle = computed(() => {
  const percentage = props.percentage;
  return {
    background: `linear-gradient(90deg, #FFEABF ${percentage-15}%, #9D7F6D ${percentage+15}%, rgba(0, 0, 0, 0%) 100%), linear-gradient(#202020, #202020)`
  };
});
</script>

<template>
  <MessageBox>
    <div class="flex flex-col w-full gap-1.5 items-center text-primary font-semibold text-sm tracking-wide">
      <div class="w-full h-1 percentage-gradient rounded-md" :style="gradientStyle"></div>
      <div class="flex w-full flex-row justify-between">
        <div>
          {{ percentage.toFixed(0) }}%
        </div>
        <div class="font-normal text-secondary">
          {{ speed }} MBPS
        </div>
      </div>
    </div>
  </MessageBox>
</template>

<style scoped lang="scss">
.percentage-gradient {
  border-radius: 0.25rem;
}
</style>