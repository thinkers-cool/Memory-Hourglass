const MAX_CONCURRENT_THUMB_LOADS = 6;

type QueueEntry = {
  run: () => void;
};

const queue: QueueEntry[] = [];
let activeLoads = 0;

function pumpThumbLoadQueue() {
  while (activeLoads < MAX_CONCURRENT_THUMB_LOADS && queue.length > 0) {
    const entry = queue.shift();
    if (!entry) {
      return;
    }
    activeLoads += 1;
    entry.run();
  }
}

export function acquireThumbLoadSlot(): Promise<void> {
  return new Promise((resolve) => {
    queue.push({ run: () => resolve() });
    pumpThumbLoadQueue();
  });
}

export function releaseThumbLoadSlot() {
  activeLoads = Math.max(0, activeLoads - 1);
  pumpThumbLoadQueue();
}

export function resetThumbLoadQueueForTests() {
  queue.length = 0;
  activeLoads = 0;
}
