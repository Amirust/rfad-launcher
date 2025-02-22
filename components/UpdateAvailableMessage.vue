<script setup lang="ts">
import MessageBox from '~/components/base/MessageBox.vue';

const props = defineProps<{
  version: string
}>();
</script>

<template>
  <MessageBox>
    <div class="flex flex-row w-full justify-between items-center text-primary font-semibold text-sm tracking-wide">
      <div>
        Доступно обновление!
      </div>
      <div>
        {{ version }}
      </div>
    </div>
  </MessageBox>
</template>

<style scoped>

</style>