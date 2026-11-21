<script setup lang="ts">
import Clock from '~/components/icons/Clock.vue';
import { AddonStatus } from '~/types/types';

export interface AddonGradient {
  from: string
  to: string
}

export interface AddonComponentProps {
  name: string
  description: string
  date: string
  status: AddonStatus
  url: string
  image?: string
  gradient?: AddonGradient
}

const props = defineProps<AddonComponentProps>()

const defaultGradient: AddonGradient = {
  from: '#FFEABF',
  to: '#D42E4F'
}

const gradientStyle = computed(() => ({
  '--addon-from': props.gradient?.from ?? defaultGradient.from,
  '--addon-to': props.gradient?.to ?? defaultGradient.to
}))

const isImageLoaded = ref(false)
const isImageFailed = ref(false)

const statusLabel = computed(() =>
  props.status === AddonStatus.Released ? 'Уже доступно' : 'Скоро'
)
</script>

<template>
  <NuxtLink :to="props.url" target="_blank" class="block w-full">
    <div class="addon-border w-full rounded-2xl p-[1.5px]" :style="gradientStyle">
      <div class="relative overflow-hidden rounded-[calc(1rem-1.5px)] bg-block">
        <div v-if="props.image" class="relative h-44 w-full bg-blockBorder">
          <div v-if="!isImageLoaded" class="absolute inset-0" :class="{ 'addon-skeleton': !isImageFailed }"/>
          <img
            :src="props.image"
            alt=""
            class="w-full h-full object-cover object-center transition-opacity duration-500"
            :class="isImageLoaded ? 'opacity-100' : 'opacity-0'"
            @load="isImageLoaded = true"
            @error="isImageFailed = true"
          >
          <div class="absolute inset-0 bg-gradient-to-b from-transparent via-transparent to-block"/>
          <span class="addon-badge absolute top-3 left-3">{{ statusLabel }}</span>
        </div>
        <div class="addon-shine"/>

        <div class="relative flex flex-col w-86 gap-2 p-4 pt-3 text-secondary text-sm tracking-wide">
          <span v-if="!props.image" class="addon-badge">{{ statusLabel }}</span>
          <div class="addon-title font-bold text-2xl leading-7">
            {{ props.name }}
          </div>
          <div class="flex flex-row gap-2 items-center font-medium">
            <Clock class="w-3.5 h-3.5"/>
            {{ props.date }}
          </div>
          <div class="font-normal text-primary whitespace-pre-line break-words">
            {{ props.description }}
          </div>
        </div>
      </div>
    </div>
  </NuxtLink>
</template>

<style scoped lang="scss">
// Градиент повторяется дважды (from → to → from → to → from) при ширине фона 200%,
// поэтому сдвиг на 100% ровно равен одному периоду и цикл склеивается без рывка
.addon-border {
  background: linear-gradient(
    90deg,
    var(--addon-from) 0%,
    var(--addon-to) 25%,
    var(--addon-from) 50%,
    var(--addon-to) 75%,
    var(--addon-from) 100%
  );
  background-size: 200% 100%;
  animation: addon-flow 4s linear infinite;
}

// Блик стартует и заканчивается за пределами карточки, поэтому перезапуск не виден
.addon-shine {
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(110deg, transparent 40%, rgba(255, 234, 191, 0.15) 50%, transparent 60%);
  background-size: 250% 100%;
  animation: addon-shine 5s linear infinite;
}

.addon-title {
  background: linear-gradient(100deg, var(--addon-from) 0%, var(--addon-to) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.addon-badge {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  width: fit-content;
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
  font-size: 0.6875rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: theme('colors.block');
  background: linear-gradient(90deg, theme('colors.primary'), #F2B35B);

  &::before {
    content: '';
    width: 0.375rem;
    height: 0.375rem;
    border-radius: 9999px;
    background: theme('colors.error');
    animation: addon-pulse 1.6s ease-in-out infinite;
  }
}

// Скелетон, пока картинка грузится
.addon-skeleton {
  background: linear-gradient(90deg, transparent 30%, rgba(255, 234, 191, 0.08) 50%, transparent 70%);
  background-size: 200% 100%;
  animation: addon-skeleton 1.4s linear infinite;
}

@keyframes addon-skeleton {
  from { background-position: 150% 0; }
  to { background-position: -50% 0; }
}

@keyframes addon-flow {
  from { background-position: 0% 0; }
  to { background-position: 100% 0; }
}

@keyframes addon-shine {
  from { background-position: 150% 0; }
  to { background-position: -50% 0; }
}

@keyframes addon-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

@media (prefers-reduced-motion: reduce) {
  .addon-border,
  .addon-shine,
  .addon-skeleton,
  .addon-badge::before {
    animation: none;
  }
}
</style>
