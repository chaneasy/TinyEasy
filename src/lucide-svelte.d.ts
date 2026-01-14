declare module 'lucide-svelte' {
  import { Component } from 'svelte';
  import { SVGAttributes } from 'svelte/elements';

  export interface IconProps extends SVGAttributes<SVGElement> {
    size?: number | string;
    color?: string;
    strokeWidth?: number | string;
    absoluteStrokeWidth?: boolean;
  }

  export class Upload extends Component<IconProps> {}
  export class FileImage extends Component<IconProps> {}
  export class CheckCircle extends Component<IconProps> {}
  export class XCircle extends Component<IconProps> {}
  export class Loader2 extends Component<IconProps> {}
  export class Play extends Component<IconProps> {}
  export class Trash2 extends Component<IconProps> {}
}
