import { X } from 'lucide-react'

export function PairModal({ onClose, onPair }: { onClose: () => void; onPair: () => void }) {
  return <div className="modal-backdrop" onMouseDown={onClose}><div className="pair-modal" onMouseDown={(event) => event.stopPropagation()}><header><div><h2>Pair new device</h2><p>Generate a lockdown pairing record over usbmuxd.</p></div><button onClick={onClose}><X size={18} /></button></header><div className="pair-body"><p><b>1</b><span><strong>Connect via USB</strong><small>A new device was detected on the muxer.</small></span></p><p><b>2</b><span><strong>Tap “Trust” on device</strong><small>Then enter the passcode below.</small></span></p><div className="passcode"><i>•</i><i>•</i><i>•</i><i /></div></div><footer><button onClick={onClose}>Cancel</button><button className="primary-button" onClick={onPair}>Generate pairing</button></footer></div></div>
}
